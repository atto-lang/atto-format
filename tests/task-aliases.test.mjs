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
for (const [name,input,expected] of cases) {
 test(name,()=>{
  const result = normalize_task_aliases(input);
  assert.equal(result,expected);
  assert.equal(normalize_task_aliases(result),result);
  assert.equal(normalize_callable_headers(input),expected);
 });
}
