/**
 * Conservative file-header admission for the lexical callable formatter.
 * It does not resolve folder names or link imports: the Rust compiler remains
 * authoritative. Never reorder or remove namespace/import declarations.
 */
export function assert_namespace_header_order(source) {
  if (typeof source !== 'string') throw new TypeError('Atto source must be a string');
  const tokens = [];
  const is_start = c => /[A-Za-z_]/.test(c || '');
  const is_tail = c => /[A-Za-z0-9_]/.test(c || '');
  let i = 0;
  while (i < source.length) {
    const c = source[i];
    if (/\s/.test(c)) { ++i; continue; }
    if (source.startsWith('//', i)) {
      const end = source.indexOf('\n', i + 2);
      i = end < 0 ? source.length : end;
      continue;
    }
    if (source.startsWith('/*', i)) {
      let depth = 1;
      i += 2;
      while (i < source.length && depth) {
        if (source.startsWith('/*', i)) { ++depth; i += 2; }
        else if (source.startsWith('*/', i)) { --depth; i += 2; }
        else ++i;
        if (depth > 128) throw new SyntaxError('E_NESTING: nested comment limit');
      }
      if (depth) throw new SyntaxError('E_EOF: unterminated block comment');
      continue;
    }
    if (c === '"' || c === "'" || c === String.fromCharCode(96)) {
      const quote = c;
      ++i;
      let closed = false;
      while (i < source.length) {
        if (source[i] === '\\') { i += 2; continue; }
        if (source[i++] === quote) { closed = true; break; }
      }
      if (!closed) throw new SyntaxError('E_EOF: unterminated literal');
      tokens.push('<literal>');
      continue;
    }
    if (is_start(c)) {
      const start = i++;
      while (is_tail(source[i])) ++i;
      tokens.push(source.slice(start, i));
    } else if (/[0-9]/.test(c)) {
      const start = i++;
      while (/[0-9]/.test(source[i] || '')) ++i;
      tokens.push(source.slice(start, i));
    } else {
      tokens.push(c);
      ++i;
    }
  }

  const is = (at, value) => tokens[at] === value;
  const ident = at => /^[A-Za-z_][A-Za-z0-9_]*$/.test(tokens[at] || '');
  const header_at = at => is(at, 'use') &&
    (is(at + 1, 'namespace') || (is(at + 1, 'default') && is(at + 2, 'namespace')));
  let seen = false;
  for (let at = 0; at < tokens.length; ++at) {
    if (!header_at(at)) continue;
    if (seen) throw new SyntaxError('E_DUPLICATE: only one namespace per file');
    if (at !== 0) throw new SyntaxError('E_SYNTAX: namespace header must precede imports and declarations');
    seen = true;
    const derived = is(at + 1, 'default');
    let j = at + (derived ? 3 : 2);
    if (derived) {
      if (is(j, '(')) {
        const depth = tokens[j + 1];
        if (!/^[0-9]+$/.test(depth || '') || !Number.isSafeInteger(Number(depth)) ||
            Number(depth) < 1 || Number(depth) > 32)
          throw new SyntaxError('E_NAMESPACE_DEPTH: expected depth from 1 to 32');
        j += 2;
        if (!is(j, ')')) throw new SyntaxError('E_SYNTAX: expected closing namespace depth parenthesis');
        ++j;
      }
    } else {
      if (!ident(j)) throw new SyntaxError('E_SYNTAX: expected namespace identifier');
      ++j;
      let depth = 1;
      while (is(j, '.')) {
        if (++depth > 32) throw new SyntaxError('E_NAMESPACE_DEPTH: at most 32 namespace segments');
        if (!ident(++j)) throw new SyntaxError('E_SYNTAX: expected namespace identifier');
        ++j;
      }
    }
    if (!is(j, ';')) throw new SyntaxError('E_SYNTAX: expected semicolon after namespace');
  }
  return source;
}
