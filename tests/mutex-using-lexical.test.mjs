// Source-only regression: this tests lexical preservation, NOT compiler lowering.
import test from 'node:test';
import assert from 'node:assert/strict';
import { normalize_callable_headers } from '../src/callable-headers.mjs';
import { normalize_ownership_operations } from '../src/ownership-operations.mjs';

const cases = [
  ['contextual using async acquisition', `
suspends fnc increment(Mutex<int> counter): int {
  using counter.lock_async() as lock_lease {
    lock_lease.value += 1;
    return lock_lease.value;
  }
}`, `
susp fnc increment(Mutex<int> counter): int {
  using counter.lock_async() as lock_lease {
    lock_lease.value += 1;
    return lock_lease.value;
  }
}`],
  ['explicit await async acquisition', `
pub suspends fnc increment(Mutex<int> counter): int {
  using await counter.lock_async() as lock_lease {
    lock_lease.value += 1;
    return lock_lease.value;
  }
}`, `
pub susp fnc increment(Mutex<int> counter): int {
  using await counter.lock_async() as lock_lease {
    lock_lease.value += 1;
    return lock_lease.value;
  }
}`],
  ['do not rewrite comments or quoted code',
    `// using await counter.lock_async() as fake { }
val source = "using counter.lock_async() as fake";
pub suspends fnc main(): int { return 0; }`,
    `// using await counter.lock_async() as fake { }
val source = "using counter.lock_async() as fake";
pub susp fnc main(): int { return 0; }`]
];

for (const [name, input, expected] of cases) {
  test(name, () => {
    const formatted = normalize_callable_headers(input);
    assert.equal(formatted, expected, 'do not insert/remove await or change lease expression');
    assert.equal(normalize_callable_headers(formatted), formatted);
    assert.equal(normalize_ownership_operations(formatted), formatted);
  });
}
