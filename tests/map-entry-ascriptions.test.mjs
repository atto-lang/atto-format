import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { normalize_callable_headers } from '../src/callable-headers.mjs';
import { normalize_ownership_operations } from '../src/ownership-operations.mjs';

const sample = await readFile(new URL('./fixtures/map-entry-ascriptions.atto', import.meta.url), 'utf8');

// Only lexical *preservation* is covered: neither function is a complete Atto
// parser or authorizes a semantic rewrite of the trailing (as Entry<...>) form.
// Full compiler-confirmed parse/format/parse is required before canonicalization.
test('both proposed ascription spellings survive existing formatter passes intact', () => {
  const first = normalize_callable_headers(sample);
  const second = normalize_ownership_operations(first);
  assert.equal(second, sample);
  assert.equal(normalize_callable_headers(first), first);
  assert.equal(normalize_ownership_operations(second), second);
  assert.ok(second.includes('("foo" => 5) as Entry<string, int>'));
  assert.ok(second.includes('"foo" => 5 (as Entry<string, int>)'));
  assert.ok(second.includes('x.add(entry1);'));
  assert.ok(second.includes('x.add(entry2);'));
  assert.ok(second.includes('x["foo"] = 3;'));
});

test('quoted pseudo-entry strings and comments remain completely unchanged', () => {
  const input = [
    '// "foo" => 5 (as Entry<string,int>)',
    'let description = "as Entry<string,int> and => are literal text";',
    'let entry = ("bar" => 7) as Entry<string,int>;',
    'let other = "bar" => 7 (as Entry<string,int>);',
  ].join('\n');
  const output = normalize_ownership_operations(normalize_callable_headers(input));
  assert.equal(output, input);
});

test('closed Record static bracket syntax is preserved; nominal Struct never gets rewritten', () => {
  const formatted = normalize_ownership_operations(normalize_callable_headers(sample));
  assert.ok(formatted.includes('let mut Record r = {'));
  assert.ok(formatted.includes('r["foo"] = 3;'));
  assert.ok(formatted.includes('r["content-type"] = 7;'));
  assert.ok(formatted.includes('expected exit: 5 (Map.add replaces; Map.try_add is insert-only)'));
  assert.ok(formatted.includes('p["foo"] = 3'), 'negative Struct syntax must remain a comment');
  assert.equal(formatted, sample);
});
