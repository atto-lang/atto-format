// Lexical formatting tests only: NOT compiler effect validation or guest parity.
import test from 'node:test';
import assert from 'node:assert/strict';
import { normalize_callable_headers } from '../src/callable-headers.mjs';

const cases = [
  ['ordinary sync creator unchanged',
    'spawns fnc make(): int { return 1; }',
    'spawns fnc make(): int { return 1; }'],
  ['suspends before spawns canonicalizes',
    'pub suspends spawns fnc main(): int { return 0; }',
    'pub susp spawns fnc main(): int { return 0; }'],
  ['reversed susp order canonicalizes',
    'pub spawns susp fnc main(): int { return 0; }',
    'pub susp spawns fnc main(): int { return 0; }'],
  ['reversed suspends order canonicalizes',
    'pub spawns suspends fnc main(): int { return 0; }',
    'pub susp spawns fnc main(): int { return 0; }'],
  ['actor receive effect modifiers',
    'define actor P as spawns suspends receive(ActorMail<int> m): void {} end',
    'define actor P as susp spawns receive(ActorMail<int> m): void {} end'],
  ['canonical actor receive unchanged',
    'define actor P as susp spawns receive(ActorMail<int> m): void {} end',
    'define actor P as susp spawns receive(ActorMail<int> m): void {} end'],
  ['bare instance-method modifier',
    'define actor P as spawns susp make(): void {} end',
    'define actor P as susp spawns make(): void {} end'],
  ['effect typed callback',
    'val callback: spawns susp fnc() -> int = factory;',
    'val callback: susp spawns fnc() -> int = factory;'],
  ['effect marked lambda',
    'val cb = spawns suspends |v| -> v;',
    'val cb = susp spawns |v| -> v;'],
  ['pure plus spawns remains invalid source unchanged',
    'pure spawns fnc forbidden(): int { return 0; }',
    'pure spawns fnc forbidden(): int { return 0; }'],
  ['duplicate markers stay visible to compiler diagnostic',
    'spawns spawns fnc bad(): int { return 0; }',
    'spawns spawns fnc bad(): int { return 0; }'],
  ['comment between effect markers blocks reordering',
    'spawns /* never discard */ suspends fnc f(): int { return 0; }',
    'spawns /* never discard */ suspends fnc f(): int { return 0; }'],
  ['comments and strings are opaque',
    '// spawns suspends fnc fake()\nval msg = "spawns suspends fnc";',
    '// spawns suspends fnc fake()\nval msg = "spawns suspends fnc";'],
  ['prefix identifiers are not modifiers',
    'pub fnc spawns_extra(): int { return 1; }',
    'pub fnc spawns_extra(): int { return 1; }'],
  ['ordinary values are not effects',
    'val spawns = 10; val suspends_extra = 5;',
    'val spawns = 10; val suspends_extra = 5;'],
];
for (const [name, input, expected] of cases) {
  test(name, () => {
    const formatted = normalize_callable_headers(input);
    assert.equal(formatted, expected);
    assert.equal(normalize_callable_headers(formatted), formatted, 'idempotent output');
  });
}
test('unknown or duplicated effects are never inferred or removed', () => {
  const examples = [
    'spawns spawns fnc x(): void {}',
    'susp spawns spawns fnc x(): void {}',
    'pure spawns fnc x(): void {}'
  ];
  for (const example of examples) {
    assert.equal(normalize_callable_headers(example), example);
  }
});
