import test from 'node:test';
import assert from 'node:assert/strict';
import { normalize_caret_sequence_spacing as format } from '../src/caret-sequences.mjs';
const cases=[
 ['return^42;', 'return ^ 42;'],
 ['return^;', 'return ^;'],
 ['return^(1,42);', 'return ^ (1,42);'],
 ['return seq!(1,42);', 'return seq! (1,42);'],
 ['^seq!(1,42);', '^ seq! (1,42);'],
 ['return seq! 1,42;', 'return seq! 1,42;'],
 ['return seq(1,42);','return seq(1,42);'],
 ['return seq (1,42);','return seq (1,42);'],
 ['return seq !(1,42);','return seq !(1,42);'],
 ['^ return 42;','^ return 42;'],
 ['return ^^ 42;','return ^^ 42;'],
 ['return (>1,42);','return (> 1,42);'],
 ['return object.seq!(1,42);','return object.seq!(1,42);'],
 ['return pkg::seq!(1,42);','return pkg::seq!(1,42);'],
 ['// seq!(1,42) return^42\nreturn "seq!(1,42) return^42";','// seq!(1,42) return^42\nreturn "seq!(1,42) return^42";'],
 ['/* seq!(1,42) */ return^seq!(1,42);','/* seq!(1,42) */ return ^ seq! (1,42);']
];
for (const [before,after] of cases) {
 test('spacing and idempotence: '+before,()=>{
  assert.equal(format(before),after);
  assert.equal(format(after),after);
 });
}

for (const source of [
 'return αseq!(1,42);',
 'return 漢seq!(1,42);',
 'return 𠀀seq!(1,42);',
 'αreturn^42;',
 'return `opaque seq!(1,42) return^42`;',
 'return `opaque ${foo(`seq!(1,42)`)}`;'
]) test('opaque identifier or template: '+source,()=>assert.equal(format(source),source));
