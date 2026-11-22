import test from 'node:test';
import assert from 'node:assert/strict';
import { normalize_ownership_operations } from '../src/ownership-operations.mjs';
import { normalize_callable_headers } from '../src/callable-headers.mjs';

// TARGET syntax only. These source-only checks protect tokens from formatter
// corruption; they DO NOT prove Atto parser acceptance, guest compilation,
// sandboxing, actor scheduling, or safe removal of explicit 'shared'.
const sample = domain => [
  `define ${domain} actor Worker receives int as`,
  '  susp receive(ActorMail<int> mail): void { }',
  'end',
  'pub fnc main(): int { return 0; }',
  '',
].join('\n');

test('actor execution modifiers are preserved by the existing lexical passes', () => {
  for (const actor of [
    sample(''), sample('shared'), sample('isolated'), sample('untrusted'),
    sample('pub isolated durable'), sample('untrusted serial http'),
  ]) {
    for (const normalize of [normalize_ownership_operations, normalize_callable_headers]) {
      assert.equal(normalize(actor), actor, 'domain header changed by lexical pass');
      assert.equal(normalize(normalize(actor)), actor, 'non-idempotent domain formatting');
    }
  }
});

test('invalid domain declarations remain invalid; formatter must not guess', () => {
  for (const actor of [
    sample('isolated untrusted'), sample('shared shared'),
    sample('durable isolated'), sample('persistent durable'),
    sample('untrusted shared'), sample('untrusted untrusted'),
  ]) {
    assert.equal(normalize_ownership_operations(actor), actor);
    assert.equal(normalize_callable_headers(actor), actor);
  }
});

test('comments, literals and nested comments cannot manufacture domain rewrites', () => {
  const source = [
    '// define untrusted actor Spoof as end',
    '/* define isolated actor Fake as /* nested define shared actor */ end */',
    'val a = "define shared actor Quote as end";',
    "val b = 'define isolated actor Quote as end';",
    'val c = `define untrusted actor Template as end`;',
    'define untrusted actor Real receives int as',
    '  susp receive(ActorMail<int> mail): void { }',
    'end',
    '',
  ].join('\n');
  for (const normalize of [normalize_ownership_operations, normalize_callable_headers]) {
    assert.equal(normalize(source), source);
    assert.equal(normalize(normalize(source)), source);
  }
});

test('mixed actor-domain and ownership operators remain distinct', () => {
  const source = [
    'define isolated actor Safe receives int as',
    '  susp receive(ActorMail<int> mail): void {',
    '    val a = borrow! mail;',
    '    val b = copy!! mail;',
    '    val c = share! mail;',
    '  }',
    'end',
  ].join('\n');
  assert.equal(normalize_ownership_operations(source), source);
});
