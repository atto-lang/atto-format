import test from 'node:test';
import assert from 'node:assert/strict';
import { normalize_callable_headers } from '../src/callable-headers.mjs';

// Lexical preservation only. This is not evidence of compiler parse or guest execution.
const source = [
  'import module buffer from pkg "std.buffer";',
  '',
  'pub fnc main(): int throws Error {',
  '  val mut bytes = buffer.ByteBuffer.allocate(8);',
  '  bytes.write_u32_le(0, 0x12345678);',
  '  val int header = bytes.read_u32_le(0);',
  '  return header eq 0x12345678 ? 1 : 0;',
  '}',
  ''
].join('\n');

test('ByteBuffer sample preserves val mut, snake_case, endian suffix, and conditional expression', () => {
  const once = normalize_callable_headers(source);
  assert.equal(once, source);
  assert.equal(normalize_callable_headers(once), once);
  assert.match(once, /val mut bytes/);
  assert.match(once, /read_u32_le\(0\)/);
  assert.match(once, /eq 0x12345678 \? 1 : 0/);
});

test('ByteBuffer API identifiers in strings and comments are not rewritten', () => {
  const snippet = '// bytes.read_u32_be(0)\nval s = "write_u32_le(0, 0x12345678)";\n';
  assert.equal(normalize_callable_headers(snippet),snippet);
});
