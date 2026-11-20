/** Conservative task statement alias normalization, target contract:
 * atto-spec/language/typed-task-payload-sync-v1.md.
 *
 * Source-only lexical transform; release use requires real compiler
 * parse/effect equivalence and (where supported) guest artifact verification.
 * Skips strings, templates, line/nested block comments and malformed syntax.
 */
export function normalize_task_aliases(source) {
  const tokens = [];
  const isHead = c => /[A-Za-z_]/.test(c ?? '');
  const isTail = c => /[A-Za-z0-9_]/.test(c ?? '');
  for (let i = 0; i < source.length;) {
    const c = source[i], start = i;
    if (/\s/.test(c)) { i++; continue; }
    if (source.startsWith('//', i)) {
      let end = source.indexOf('\n', i);
      i = end < 0 ? source.length : end;
      continue;
    }
    if (source.startsWith('/*', i)) {
      let depth = 1;
      i += 2;
      while (i < source.length && depth) {
        if (source.startsWith('/*', i)) { depth++; i += 2; }
        else if (source.startsWith('*/', i)) { depth--; i += 2; }
        else i++;
      }
      if (depth) return source;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      const quote = c;
      i++;
      while (i < source.length) {
        if (source[i] === '\\') { i += 2; continue; }
        if (source[i++] === quote) break;
      }
      if (source[i - 1] !== quote) return source;
      continue;
    }
    if (isHead(c)) {
      i++;
      while (i < source.length && isTail(source[i])) i++;
    } else {
      i++;
    }
    tokens.push({text:source.slice(start,i),start,end:i});
  }
  const at=(i,s)=>tokens[i]?.text===s;
  const ws=(a,b)=> /^\s*$/.test(source.slice(a,b));
  function paired(atIndex, open, close) {
    if (!at(atIndex,open)) return -1;
    let depth=0;
    for (let i=atIndex;i<tokens.length;i++) {
      if (at(i,open)) depth++;
      else if (at(i,close) && --depth===0) return i+1;
    }
    return -1;
  }
  const edits = [];
  for (let i=0;i+3<tokens.length;i++) {
    if (!at(i,'do') || !at(i+1,'sync') || !at(i+2,'task')) continue;
    if (!ws(tokens[i].end,tokens[i+1].start) || !ws(tokens[i+1].end,tokens[i+2].start)) continue;
    let j=i+3;
    if (at(j,'<')) j=paired(j,'<','>');
    if (j<0 || !at(j,'{')) continue;
    if (paired(j,'{','}')<0) continue;
    // Keep 'do' and 'task' and their source positions; remove only sync
    // and its trailing whitespace. Do not change comments or task payloads.
    edits.push({start:tokens[i+1].start,end:tokens[i+2].start});
  }
  let result=source;
  for (const edit of edits.reverse())
    result=result.slice(0,edit.start)+result.slice(edit.end);
  return result;
}
