// Host-only consumer fence: not a general parser, semantic AST proof, or guest receipt.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {normalize_ownership_operations as normalize} from '../src/ownership-operations.mjs';
const registry = JSON.parse(readFileSync(new URL('../contracts/spec-rule-consumer-v1.json', import.meta.url),'utf8'));

test('consumer references canonical pinned spec without claiming full compliance', () => {
  assert.equal(registry.schema,'atto.format.spec-consumer.v1');
  assert.match(registry.canonical_spec.commit,/^[a-f0-9]{40}$/);
  assert.match(registry.canonical_spec.registry_blob_sha,/^[a-f0-9]{40}$/);
  assert.equal(registry.canonical_spec.repository,'atto-lang/atto-spec');
  assert.equal(new Set(registry.rule_ids).size,registry.rule_ids.length);
  for (const id of ['ATTO-OWN-001','ATTO-OWN-002','ATTO-OWN-003','ATTO-FMT-001','ATTO-FMT-002','ATTO-FMT-003'])
    assert.ok(registry.rule_ids.includes(id));
  assert.equal(registry.evidence.full_parser,'not_executed');
  assert.equal(registry.evidence.independent_semantic_ast,'not_executed');
  assert.equal(registry.evidence.compiled_guest_differential,'not_executed');
});

test('ownership lexical pass retains distinct operators, literal and comments', () => {
  const s = [
    'val one = copy! x;',
    'val two = copy!! x;',
    'val three = copy! !x;',
    'val four = take! x;',
    'drop! x;',
    'define const string api_host as os.read_env("lifetime! a");',
    '// lifetime! b',
    '/* lifetime! c */',
    'fnc inspect<lifetime! a, T>(Borrow<T, lt=a> x) -> void {}',
  ].join('\n');
  const f = normalize(s);
  assert.ok(f.includes('copy! x;'));
  assert.ok(f.includes('copy!! x;'));
  assert.ok(f.includes('copy! !x;'));
  assert.ok(f.includes('take! x;'));
  assert.ok(f.includes('drop! x;'));
  assert.ok(f.includes('"lifetime! a"'));
  assert.ok(f.includes('// lifetime! b'));
  assert.ok(f.includes('/* lifetime! c */'));
  assert.ok(f.includes('fnc inspect<lt! a, T>'));
  assert.equal(normalize(f),f);
});

test('isolated normalizer does not silently reorder or repair namespace errors', () => {
  const invalid = [
    'import * as math from "./math.atto";',
    'use namespace storefront;',
    'use namespace storefront;'
  ].join('\n');
  assert.equal(normalize(invalid),invalid);
  // This tests NON-REWRITE only; a real compiler still must reject invalid.
});
