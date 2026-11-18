import test from 'node:test';
import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { normalize_ownership_operations as normalize } from '../src/ownership-operations.mjs';

test('dense 780 KiB lifetime binders normalize in bounded linear time', () => {
  // Regression: repeated source.slice(0, tokenStart) and repeated output
  // prefix rebuilding turned 20K tokens into multi-second quadratic work.
  const n = 60_000;
  const input = 'lifetime! a;\n'.repeat(n);
  const started = performance.now();
  const formatted = normalize(input);
  const elapsed = performance.now() - started;
  assert.equal(formatted, 'lt! a;\n'.repeat(n));
  assert.ok(elapsed < 5_000, `unbounded formatter time: ${elapsed}ms`);
  assert.equal(normalize(formatted), formatted);
});

test('one long line with 60K tokens stays linear as well', () => {
  const input = 'lifetime! a; '.repeat(60_000);
  const started = performance.now();
  const normalized = normalize(input);
  assert.equal(normalized, 'lt! a; '.repeat(60_000));
  assert.ok(performance.now() - started < 5_000, 'single-line scan must remain bounded');
});

test('at source cap rejects excessive input before parsing', () => {
  assert.throws(() => normalize('x'.repeat(1_048_577)), RangeError);
});

test('adjacent bang tokens preserve all ownership operations and boundaries', () => {
  const ops = ['copy!!value', 'copy! !value', 'borrow! mut value', 'take! value',
    'share! value', 'freeze! take! value', 'weak! handle', 'pin! handle',
    'drop! handle', 'lt! a'];
  for (const op of ops) assert.equal(normalize(op), op);
});

test('nested comments with fake delimiters do not rewrite operators', () => {
  const source = [
    '/* lifetime! hidden /* lifetime! nested */ lifetime! still_hidden */',
    '// lifetime! comment',
    'lifetime!! false;',
    'obj.lifetime! x;',
    'fnc f<lifetime! valid>() {}',
  ].join('\n');
  assert.equal(normalize(source), source.replace('<lifetime! valid>', '<lt! valid>'));
});

test('untrusted unicode or malformed binder identifiers remain unchanged', () => {
  const bad = ['lifetime! 2', 'lifetime! 𝛼', 'lifetime! /*comment*/ x',
    'lifetime!! alpha'];
  for (const source of bad) assert.equal(normalize(source), source);
});
