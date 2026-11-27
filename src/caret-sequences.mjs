/**
 * Conservative, independent spacing normalizer for the AttoLang `^` return
 * token and `(>` delimited sequence opener.
 *
 * This is NOT a parser, migration tool, or general-purpose formatter.
 * No `return` -> `^`, `seq` -> `begin`, or `-> T` -> `-> T ::`
 * conversion is authorized. Caller MUST use the guest compiler verifier
 * before persisting changes.
 */
export function normalize_caret_sequence_spacing(source) {
  if (typeof source !== 'string') throw new TypeError('Atto source string required');
  if (Buffer.byteLength(source, 'utf8') > 1024 * 1024) throw new RangeError('Atto source exceeds 1 MiB');

  const wordStart = c => /[A-Za-z_]/.test(c || '');
  const wordTail = c => /[A-Za-z0-9_]/.test(c || '');
  const edits = [];
  let previous = null;

  function statementPosition() {
    return previous === null ||
      ['{', '}', ';', 'then', 'else', 'do', 'fi', '->'].includes(previous);
  }
  for (let i = 0; i < source.length;) {
    const c = source[i];
    if (/\s/.test(c)) { ++i; continue; }

    if (source.startsWith('//', i)) {
      const end = source.indexOf('\n', i + 2);
      i = end < 0 ? source.length : end;
      continue;
    }
    if (source.startsWith('/*', i)) {
      const start = i;
      let depth = 1; i += 2;
      while (i < source.length && depth) {
        if (source.startsWith('/*', i)) { depth++; i += 2; }
        else if (source.startsWith('*/', i)) { depth--; i += 2; }
        else ++i;
      }
      if (depth) throw new Error('Unterminated nested comment at offset ' + start);
      continue;
    }
    if (c === '"' || c === "'") {
      const quote = c, start = i++;
      let closed = false;
      while (i < source.length) {
        if (source[i] === '\\') { i += 2; continue; }
        if (source[i++] === quote) { closed = true; break; }
      }
      if (!closed) throw new Error('Unterminated string/character literal at offset ' + start);
      previous = 'literal';
      continue;
    }
    if (wordStart(c)) {
      const start = i++;
      while (wordTail(source[i])) ++i;
      const word = source.slice(start, i);
      // seq! is a contiguous construct marker. Only normalize the
      // unambiguous delimited opener here; never join a separated bang
      // or rewrite an ordinary seq(...) function call.
      if (word === 'seq' && source.startsWith('!(', i) &&
          previous !== '.' && previous !== '::') {
        edits.push({at:i+1, text:' '});
        previous = 'seq!'; ++i; continue;
      }
      previous = word;
      continue;
    }
    if (/[0-9]/.test(c)) {
      ++i; while (/[0-9_]/.test(source[i] || '')) ++i;
      previous = 'number';
      continue;
    }

    if (source.startsWith('^^', i) || source.startsWith('^=', i)) {
      previous = source.slice(i, i+2); i += 2; continue;
    }
    if (c === '^') {
      const next = source[i+1];
      // The literal return followed by ^ is redundant, not a sequence
      // selector. Preserve both tokens for compiler diagnostics and
      // normalize only spacing; never rewrite the tuple operand.
      const redundantReturn = previous === 'return';
      if (redundantReturn && i > 0 && !/\s/.test(source[i-1])) {
        edits.push({at:i, text:' '});
      }
      if ((statementPosition() || redundantReturn) && next !== undefined && next !== ';' &&
          next !== '^' && next !== '=' && !/\s/.test(next) &&
          !source.startsWith('//', i+1) && !source.startsWith('/*', i+1)) {
        edits.push({at:i+1, text:' '});
      }
      previous = '^'; ++i; continue;
    }
    if (source.startsWith('(>', i)) {
      const next = source[i+2];
      if (next !== undefined && next !== ')' && !/\s/.test(next) &&
          !source.startsWith('/*', i+2) && !source.startsWith('//', i+2)) {
        edits.push({at:i+2, text:' '});
      }
      previous = '(>'; i += 2; continue;
    }
    if (source.startsWith('::', i) || source.startsWith('->', i) ||
        source.startsWith('=>', i) || source.startsWith('>>', i)) {
      previous = source.slice(i, i+2); i += 2; continue;
    }

    previous = c;
    ++i;
  }
  let formatted = source;
  for (let i=edits.length-1; i>=0; --i) {
    const e=edits[i];
    formatted = formatted.slice(0,e.at) + e.text + formatted.slice(e.at);
  }
  return formatted;
}
