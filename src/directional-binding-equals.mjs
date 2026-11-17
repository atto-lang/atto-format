/**
 * Conservative, lexical AttoLang directional-binding normalizer.
 *
 * Rewrites a redundant, separated '= ' only after a contiguous '>>'
 * and immediately before a declaration binder (val/let) or a contextual
 * Future<T> slot in an nb statement.
 *
 * This is a candidate formatter, not a type checker. Use the compiler-backed
 * verified wrapper before applying it to executable source. In particular,
 * stdin guest code cannot yet pass that verification gate.
 */
export function normalize_directional_binding_equals(source) {
  if (typeof source !== 'string') throw new TypeError('AttoLang source must be a string');
  if (source.length > 1024 * 1024)
    throw new RangeError('AttoLang source exceeds directional formatter size limit');

  const toks = [];
  const isHead = c => c != null && /[\p{L}_]/u.test(c);
  const isTail = c => c != null && /[\p{L}\p{N}_]/u.test(c);
  for (let i = 0; i < source.length;) {
    const start = i;
    const ch = source[i];
    if (/\s/u.test(ch)) { ++i; continue; }
    if (source.startsWith('//', i)) {
      const newline = source.indexOf('\n', i + 2);
      i = newline < 0 ? source.length : newline;
      continue;
    }
    if (source.startsWith('/*', i)) {
      let depth = 1;
      i += 2;
      while (depth && i < source.length) {
        if (source.startsWith('/*', i)) { ++depth; i += 2; }
        else if (source.startsWith('*/', i)) { --depth; i += 2; }
        else ++i;
        if (depth > 64) throw new RangeError('Block comment nesting exceeds formatter limit');
      }
      if (depth) throw new Error('Unterminated block comment');
      continue;
    }
    // Backtick templates may contain interpolation and nested source-like
    // text. Until the formatter has a real template scanner, reject them
    // rather than rewriting anything inside a template literal.
    if (ch === '`') throw new Error('Template literals require a full Atto lexer');
    if (ch === '"' || ch === "'") {
      const quote = ch;
      ++i;
      let done = false;
      while (i < source.length) {
        if (source[i] === '\\') { i += 2; continue; }
        if (source[i] === quote) { ++i; done = true; break; }
        if (source[i] === '\n') break;
        ++i;
      }
      if (!done) throw new Error('Unterminated string literal');
      toks.push({ s: source.slice(start, i), start, end: i, literal: true });
      continue;
    }
    if (isHead(ch)) {
      ++i;
      while (i < source.length && isTail(source[i])) ++i;
    } else if (['>>=', '>>', '<<', '|>', '<|', '==', '>=', '<=', '->', '=>', '::'].some(op => {
      if (!source.startsWith(op, i)) return false;
      i += op.length;
      return true;
    })) {
      // Longest operators are visited first, so contiguous '>>=' is
      // never mistaken for '>>' followed by the optional equals marker.
    } else {
      ++i;
    }
    toks.push({ s: source.slice(start, i), start, end: i, literal: false });
    if (toks.length > 100_000)
      throw new RangeError('Too many tokens for directional formatter');
  }

  const isIdentifier = token => token && !token.literal &&
    /^[\p{L}_][\p{L}\p{N}_]*$/u.test(token.s);
  const edits = [];
  let angleDepth = 0;
  for (let i = 0; i < toks.length; ++i) {
    const t = toks[i];
    if (t.s === ';' || t.s === '{' || t.s === '}') {
      angleDepth = 0;
      continue;
    }
    if (t.s === '<') { ++angleDepth; continue; }
    if (t.s === '>') { angleDepth = Math.max(0, angleDepth - 1); continue; }
    if (t.s !== '>>') continue;
    if (angleDepth) {
      angleDepth = Math.max(0, angleDepth - 2);
      continue; // nested generic type closer, never a directional capture
    }
    const eq = toks[i + 1], binder = toks[i + 2];
    if (eq?.s !== '=' || !binder || binder.literal) continue;
    if (!['val', 'let', 'Future'].includes(binder.s)) continue;
    const left = source.slice(t.end, eq.start);
    const right = source.slice(eq.end, binder.start);
    if (!/^\s+$/u.test(left) || !/^\s*$/u.test(right)) continue;
    const preceding = toks[i - 1];
    if (!isIdentifier(preceding) && ![')', ']', '?'].includes(preceding?.s)) continue;

    if (binder.s === 'Future') {
      if (toks[i + 3]?.s !== '<') continue;
      // A type slot is not an expression. This normalization applies only
      // when the statement is an explicit nonblocking read registration.
      let nb = false;
      for (let j = i - 1; j >= 0; --j) {
        if ([';', '{', '}'].includes(toks[j].s)) break;
        if (toks[j].s === 'nb') { nb = true; break; }
      }
      if (!nb) continue;
    }
    const whitespace = left.includes('\n') ? left :
      right.includes('\n') ? right : ' ';
    edits.push({ start: t.end, end: binder.start, replacement: whitespace });
  }

  let out = source;
  for (const { start, end, replacement } of edits.reverse()) {
    out = out.slice(0, start) + replacement + out.slice(end);
  }
  return out;
}
