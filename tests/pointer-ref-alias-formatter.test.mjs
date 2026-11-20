import test from 'node:test';
import assert from 'node:assert/strict';
import { normalize_ownership_operations as normalize } from '../src/ownership-operations.mjs';

// Formatter-only assertions. These do not certify that any Atto compiler
// accepts Pointer<int> or executes a guest, which remains an independent gate.
const rt = `pub fnc main(): int {
  let value: int = 42;
  val Pointer<int> ptr = rt ref value;
  val answer = rt deref ptr;
  rt unref ptr;
  return answer;
}
`;
const bang = `pub fnc main(): int {
  let value: int = 42;
  val Pointer<int> ptr = rt ref value;
  val answer = deref! ptr;
  unref! ptr;
  return answer;
}
`;

test('keep exact Pointer<int> local rt and bang programs intact', () => {
  for (const source of [rt, bang]) {
    assert.equal(normalize(source), source);
    assert.equal(normalize(normalize(source)), source);
  }
  assert.equal(rt.replace('rt deref ptr', 'deref! ptr').replace('rt unref ptr', 'unref! ptr'), bang);
});

test('never silently rewrite rt ref/deref/unref to a different ownership category', () => {
  const examples = [
    'val Pointer<int> p = rt ref x;',
    'val Pointer<int> p = ref! x;',
    'val a = rt deref p;',
    'val a = deref! p;',
    'rt unref p;',
    'unref! p;',
    'val a = deref!p;',
    'unref!p;',
    'val a = ref!p;',
    'val a = borrow! p;',
  ];
  for (const text of examples) assert.equal(normalize(text), text, text);
});

test('preserve literals/comments and avoid inventing live pointer references', () => {
  const source = [
    '// rt ref x; deref! ptr; unref! ptr;',
    'val example = "rt ref x; deref! ptr";',
    '/* rt unref ptr; ref! x */',
    'val a = ref! original;',
  ].join('\n');
  assert.equal(normalize(source), source);
});
