import test from 'node:test';
import assert from 'node:assert/strict';
import {normalize_caret_sequence_spacing}
  from '../src/caret-sequences.mjs';

// Formatter preservation target, NOT a proof of compiler or guest support.
const source="pub suspends fnc next_text(): string using io, suspend {\n  val input = stdio.in;\n  val output = stdio.out;\n  return match writech output, \"prompt\" over\n    on is Ok<_> -> match << input over\n      on is Ok<line> -> match line over\n        on is Some<item> -> item,\n        on None -> \"\",\n      end,\n      on is Err<error> -> \"\",\n    end,\n    on is Err<error> -> \"\",\n  end;\n}\n";

test('caret/sequence formatter never rewrites nominal is-ADT binders or unary stdin read',()=>{
  const first=normalize_caret_sequence_spacing(source);
  assert.equal(first,source,'existing checked match syntax must remain token-exact');
  assert.equal(normalize_caret_sequence_spacing(first),first,'idempotence');
  for(const token of ['on is Ok<_>','on is Ok<line>',
                      'on is Some<item>','on is Err<error>',
                      'on None','match << input over']) {
    assert.ok(first.includes(token),token);
  }
});
test('comments and strings containing constructor-shaped text are opaque',()=>{
  const code='// on is Ok<_> -> match << input\n'+
    'pub fnc main(): int { val s="on is Some<item>"; ^0; }';
  assert.equal(normalize_caret_sequence_spacing(code),code.replace('^0','^ 0'));
});
