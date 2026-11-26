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
suspends fnc increment(Mutex<int> counter): int {
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
pub suspends fnc increment(Mutex<int> counter): int {
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
pub suspends fnc main(): int { return 0; }`]
];

for (const [name, input, expected] of cases) {
  test(name, () => {
    const formatted = normalize_callable_headers(input);
    assert.equal(formatted, expected, 'do not insert/remove await or change lease expression');
    assert.equal(normalize_callable_headers(formatted), formatted);
    assert.equal(normalize_ownership_operations(formatted), formatted);
  });
}


test('formatter preserves mandatory new on opaque Mutex resource class', () => {
  const cases = [
    'val counter = new Mutex<int>(0);',
    'val counter = new Mutex<Map<int, string>>(Map.of());',
    'val counter = Mutex<int>(0); // invalid; do NOT silently insert new',
    'val p = Point(x=1, y=2); // ordinary value struct, no new required',
    'val source = "Mutex<int>(0)"; // quoted old source is not repaired',
    '// new Mutex<int>(0) is a class constructor in a comment',
  ];
  for (const src of cases) {
    const once=normalize_callable_headers(src);
    assert.equal(once,src);
    assert.equal(normalize_callable_headers(once),once);
    assert.equal(normalize_ownership_operations(once),once);
  }
});

test('effect aliases, class constructor and scope remain byte-identical', () => {
  const input=[
    'suspends fnc increment(Mutex<int> counter): int {',
    '  using await counter.lock_async() as lease {',
    '    lease.value += 1;',
    '    return lease.value;',
    '  }',
    '}',
    'pub suspends fnc main(): int {',
    '  val counter = new Mutex<int>(0);',
    '  return await increment(counter);',
    '}',
  ].join('\n');
  const expected=input;
  assert.equal(normalize_callable_headers(input),expected);
});
