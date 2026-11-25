import test from 'node:test';
import assert from 'node:assert/strict';
import { assert_namespace_header_order } from '../src/namespace-header-guard.mjs';
import { normalize_callable_headers } from '../src/callable-headers.mjs';

const valid = [
  'pub fnc main(): int { return 0; }',
  '// use namespace fake;\nuse namespace storefront;\nimport module math from "./math.atto";\npub fnc main(): int { return 0; }',
  '/* use namespace fake; */ use namespace shop.frontend; import x from pkg "std.time";',
  'use default namespace;\nimport module math from "./math.atto";',
  'use default namespace(2);\nimport span, foo as bar from pkg "core.telemetry";',
  'use default namespace(32);',
  'use namespace a.b.c;\ndefine class A as fnc x() -> int :: 1 end end',
  'pub fnc main(): int { stdio.println("use namespace hidden;"); return 0; }',
  '/* outer /* use default namespace; */ outer */ use namespace real;'
];
for (const source of valid) test('preserves valid source header '+source.slice(0,35), () => {
  assert.equal(assert_namespace_header_order(source), source);
  assert.equal(assert_namespace_header_order(assert_namespace_header_order(source)), source);
});
const invalid = [
  ['import module math from "./math.atto"; use namespace storefront;', 'E_SYNTAX'],
  ['pub fnc main(): int {return 0;} use namespace storefront;', 'E_SYNTAX'],
  ['use namespace storefront; use namespace other;', 'E_DUPLICATE'],
  ['use default namespace(2); use namespace storefront;', 'E_DUPLICATE'],
  ['use namespace storefront; use default namespace;', 'E_DUPLICATE'],
  ['use namespace x.;', 'E_SYNTAX'],
  ['use namespace;', 'E_SYNTAX'],
  ['use namespace x', 'E_SYNTAX'],
  ['use default namespace(0);', 'E_NAMESPACE_DEPTH'],
  ['use default namespace(33);', 'E_NAMESPACE_DEPTH'],
  ['use default namespace();', 'E_NAMESPACE_DEPTH'],
  ['use default namespace(2', 'E_SYNTAX'],
  ['use default namespace(1.5);', 'E_SYNTAX'],
  ['use namespace x.y. ;', 'E_SYNTAX'],
  ['/* incomplete use namespace x;', 'E_EOF']
];
for (const [source, code] of invalid) test('rejects '+source.slice(0,38), () => {
  assert.throws(() => assert_namespace_header_order(source), new RegExp(code));
  assert.throws(() => normalize_callable_headers(source), new RegExp(code),
    'formatter must refuse unsafe edits before callable normalization');
});
test('formatter preserves valid namespaces and import ordering byte-for-byte', () => {
  for (const source of valid.filter(s => s.includes('use namespace') || s.includes('use default namespace'))) {
    if (!source.startsWith('use ') && !source.startsWith('//') && !source.startsWith('/*')) continue;
    const output = normalize_callable_headers(source);
    assert.equal(output.slice(0, source.indexOf(';') + 1), source.slice(0, source.indexOf(';') + 1));
    assert.equal(normalize_callable_headers(output), output);
  }
});
test('valid namespace with callable rewrite retains its original header', () => {
  const source = 'use namespace storefront;\nimport module math from "./math.atto";\ndefine class C as fnc answer(): int { return 42; } end';
  const expected = 'use namespace storefront;\nimport module math from "./math.atto";\ndefine class C as answer(): int { return 42; } end';
  assert.equal(normalize_callable_headers(source), expected);
  assert.equal(normalize_callable_headers(expected), expected);
});
