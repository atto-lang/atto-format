import test from 'node:test';
import assert from 'node:assert/strict';
import {normalize_task_aliases} from '../src/task-aliases.mjs';
import {normalize_callable_headers} from '../src/callable-headers.mjs';

const cases = [
 ['typed', 'do sync task<int> { return 42; };', 'do task<int> { return 42; };'],
 ['inferred', 'do sync task { work(); };', 'do task { work(); };'],
 ['void', 'do sync task<void> { return; };', 'do task<void> { return; };'],
 ['nested Future type', 'do sync task<Future<int>> { return task<int> { return 42; }; };',
   'do task<Future<int>> { return task<int> { return 42; }; };'],
 ['deep generic', 'do sync task<Result<Future<int>, Error>> { return work(); };',
   'do task<Result<Future<int>, Error>> { return work(); };'],
 ['standalone sync join', 'val int n = sync task<int> { return 42; };',
   'val int n = sync task<int> { return 42; };'],
 ['await stays explicit', 'val int n = await task<int> { return 42; };',
   'val int n = await task<int> { return 42; };'],
 ['already canonical', 'do task<int> { return 42; };',
   'do task<int> { return 42; };'],
 ['ordinary block remains lexical', 'block { work(); }', 'block { work(); }'],
 ['prefix identifier', 'do synchronize task<int> { return 42; };',
   'do synchronize task<int> { return 42; };'],
 ['separated comment', 'do /* important */ sync task<int> { return 42; };',
   'do /* important */ sync task<int> { return 42; };'],
 ['second separated comment', 'do sync /* important */ task<int> { return 42; };',
   'do sync /* important */ task<int> { return 42; };'],
 ['string', 'val s = "do sync task<int> { return 42; }";',
   'val s = "do sync task<int> { return 42; }";'],
 ['single quote', "val s = 'do sync task<int> { return 42; }';",
   "val s = 'do sync task<int> { return 42; }';"],
 ['template', 'val s = `do sync task<int> { return 42; }`;',
   'val s = `do sync task<int> { return 42; }`;'],
 ['line comment', '// do sync task<int> { return 42; };\ndo sync task<int> { return 1; };',
   '// do sync task<int> { return 42; };\ndo task<int> { return 1; };'],
 ['nested comment', '/* do /* sync task<int> */ do sync task<int> */\ndo sync task<int> { return 1; };',
   '/* do /* sync task<int> */ do sync task<int> */\ndo task<int> { return 1; };'],
 ['malformed body', 'do sync task<int> { return 42;', 'do sync task<int> { return 42;'],
 ['malformed type', 'do sync task<Future<int> { return 42; };',
   'do sync task<Future<int> { return 42; };'],
];
// Historical 'expected' values record the forbidden rewrite that the
// previous formatter performed. Different scheduler semantics make that
// rewrite illegal even when return values happen to coincide.
for (const [name,input,legacyExpected] of cases) {
 test(name,()=>{
  const result = normalize_task_aliases(input);
  assert.equal(result,input,'task alias normalizer must preserve sync identity');
  assert.equal(normalize_task_aliases(result),result,'normalization is idempotent');
  assert.equal(normalize_callable_headers(input),input,
    'callable-header formatting must not turn inline into detached work');
  if (input !== legacyExpected) {
    assert.notEqual(result,legacyExpected,
      'old do-sync-to-do-task rewrite must never reappear');
  }
 });
}

test('inline and independent task effects are distinct through full source formatting',()=>{
 const source = `pub fnc main(): int {
  do sync task<void> { return; };
  do task<int> { return 2; };
  val int value = sync task<int> { return 42; };
  return value;
}`;
 const formatted = normalize_callable_headers(source);
 assert.match(formatted,/do sync task<void>/);
 assert.match(formatted,/do task<int>/);
 assert.match(formatted,/val int value = sync task<int>/);
 assert.equal((formatted.match(/do sync task<void>/g) ?? []).length,1);
 assert.equal((formatted.match(/do task<int>/g) ?? []).length,1);
 assert.equal(normalize_callable_headers(formatted),formatted);
});

test('canonical void-only do sync task and typed inline value remain unchanged',()=>{
 const source = `pub fnc main(): int {
  do sync task {};
  val result = sync task<int> {
    return 40 + 2;
  };
  do sync task<void> {};
  return result;
}`;
 const formatted = normalize_callable_headers(source);
 assert.equal(formatted,source);
 assert.equal(normalize_task_aliases(formatted),source);
 assert.match(formatted,/do sync task<void> \{\};/);
 assert.match(formatted,/val result = sync task<int>/);
 assert.doesNotMatch(formatted,/do task<void>/);
 // Invalid typed do-inline must remain invalid, never be coerced into void.
 const invalid = 'do sync task<int> { return 42; };';
 assert.equal(normalize_task_aliases(invalid),invalid);
});
