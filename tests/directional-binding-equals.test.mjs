import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { normalize_directional_binding_equals as format } from '../src/directional-binding-equals.mjs';
import { normalize_verified_directional_binding_equals as verify } from '../src/verified-directional-binding-equals.mjs';

const cases = [
  [
    'exact named Future user example with two optional equals',
    'nb stdio.in >> = Future<Result<Option<string>, IoError>> x\n' +
      '    |> |f| -> (await f)?\n' +
      '    >> = val Option<string> pending_line;',
    'nb stdio.in >> Future<Result<Option<string>, IoError>> x\n' +
      '    |> |f| -> (await f)?\n' +
      '    >> val Option<string> pending_line;'
  ],
  ['first binder only', 'nb stdio.in >> = Future<int> pending |> |f| -> await f >> val int x;',
    'nb stdio.in >> Future<int> pending |> |f| -> await f >> val int x;'],
  ['last capture only', 'pending |> susp |f| -> await f >> = val result: int;',
    'pending |> susp |f| -> await f >> val result: int;'],
  ['plain readable binding', 'inbox >> = val message;', 'inbox >> val message;'],
  ['select arm', 'do select { when inbox >> = let message -> {} }',
    'do select { when inbox >> let message -> {} }'],
  ['caret return', '^ pending |> susp |f| -> await f >> = val x: int;',
    '^ pending |> susp |f| -> await f >> val x: int;'],
  ['reverse lambda remains unchanged',
    'nb stdio.in >> (val string x << |f| -> await f <| Future<string>);',
    'nb stdio.in >> (val string x << |f| -> await f <| Future<string>);'],
  ['contiguous shift-assignment remains distinct', 'inbox >>= val value;', 'inbox >>= val value;'],
  ['not equality', 'inbox >> == val value;', 'inbox >> == val value;'],
  ['not repeated equals', 'inbox >> = = val value;', 'inbox >> = = val value;'],
  ['not an arbitrary assignment', 'inbox >> = value;', 'inbox >> = value;'],
  ['not numeric shift', '42 >> = val value;', '42 >> = val value;'],
  ['not separated direction', 'inbox > > = val value;', 'inbox > > = val value;'],
  ['not an ordinary Future expression', 'x >> = Future<int> pending;', 'x >> = Future<int> pending;'],
  ['not comments or quoted strings',
    '// inbox >> = val x;\nval x = "inbox >> = val x;";\n/* nested /* source >> = val a */ x >> = val b */',
    '// inbox >> = val x;\nval x = "inbox >> = val x;";\n/* nested /* source >> = val a */ x >> = val b */'],
  ['comments between direction and equals block edits',
    'inbox >> /* keep */ = val value;', 'inbox >> /* keep */ = val value;'],
  ['generic close is never captured',
    'val a: Result<Option<string>,IoError>> = val x;',
    'val a: Result<Option<string>,IoError>> = val x;'],
  ['line breaks in the direction marker retain line structure',
    'nb stdio.in >>\n    = Future<string> x |> |f| -> await f >> =\n    val string line;',
    'nb stdio.in >>\n    Future<string> x |> |f| -> await f >>\n    val string line;'],
  ['quoted escaped markers remain literal',
    'val x = "quoted \\" nb stdio.in >> = Future<int> x"; inbox >> = val y;',
    'val x = "quoted \\" nb stdio.in >> = Future<int> x"; inbox >> val y;'],
];
for (const [name, input, expected] of cases) {
  test(name, () => {
    const actual = format(input);
    assert.equal(actual, expected);
    assert.equal(format(actual), actual, 'format twice equals format once');
  });
}
test('unterminated comments/strings and oversized source fail closed', () => {
  assert.throws(() => format('inbox >> = val x; /*'), /Unterminated/);
  assert.throws(() => format('inbox >> = val x; "'), /Unterminated/);
  assert.throws(() => format('x'.repeat(1024 * 1024 + 1)), RangeError);
});
const magic = [0, 97, 115, 109, 1, 0, 0, 0];
function mockEquivalentGuest(text) {
  const digest = createHash('sha256').update(format(text)).digest();
  return Uint8Array.from([...magic, ...digest]);
}
test('verified wrapper requires identical guest bytes and checks twice for a changed source', async () => {
  const src='nb stdio.in >> = Future<int> x |> |f| -> await f >> = val int y;';
  let calls = 0;
  const result = await verify(src, async text => {
    calls++;
    return mockEquivalentGuest(text);
  });
  assert.equal(calls, 2);
  assert.equal(result.changed, true);
  assert.equal(result.guest_sha256_same, true);
  assert.equal(result.formatted, format(src));
});
test('verified wrapper refuses incompatible Wasm, fake Wasm, and unsupported compiler', async () => {
  const src = 'inbox >> = val x;';
  let calls = 0;
  await assert.rejects(verify(src, async () =>
    Uint8Array.from([...magic, calls++])), /guest Wasm changed/);
  await assert.rejects(verify(src, async () => new Uint8Array(8)), /valid guest Wasm/);
  await assert.rejects(verify(src, async () => { throw new Error('E_UNSUPPORTED'); }), /E_UNSUPPORTED/);
  await assert.rejects(verify(src, null), TypeError);
});
test('verified wrapper does not recompile an unchanged source', async () => {
  let calls=0;
  const result=await verify('inbox >> val x;', async text => { calls++; return mockEquivalentGuest(text); });
  assert.equal(calls, 1);
  assert.equal(result.changed, false);
});
