import test from 'node:test';
import assert from 'node:assert/strict';
import {normalize_caret_sequence_spacing} from '../src/caret-sequences.mjs';
import {normalize_verified_caret_sequences} from '../src/verified-caret-sequences.mjs';

const canonical=[
  ['caret scalar', 'pub fnc main(): int { ^42; }', 'pub fnc main(): int { ^ 42; }'],
  ['caret parenthesized expression', 'pub fnc main(): int { ^(40+2); }', 'pub fnc main(): int { ^ (40+2); }'],
  ['caret seq', 'pub fnc main(): int { ^seq let a = 5, a + 3; }', 'pub fnc main(): int { ^ seq let a = 5, a + 3; }'],
  ['nested paren sequences', 'pub fnc main(): int { ^(>let x=5,(>let y=x+3,y)); }',
   'pub fnc main(): int { ^ (> let x=5,(> let y=x+3,y)); }'],
  ['then-arm caret', 'fnc f(): int { if true then ^42; else ^0; fi }',
   'fnc f(): int { if true then ^ 42; else ^ 0; fi }'],
  ['lambda body caret', 'fnc f(): int { const f = |x| -> ^x+1; ^5; }',
   'fnc f(): int { const f = |x| -> ^ x+1; ^ 5; }'],
  ['else-free guard caret exits',
   'pub fnc main(): int { guard false { ^-1; } ^42; }',
   'pub fnc main(): int { guard false { ^ -1; } ^ 42; }'],
  ['guard true preserves fallthrough and caret return',
   'pub fnc main(): int { guard true { ^-1; } ^42; }',
   'pub fnc main(): int { guard true { ^ -1; } ^ 42; }'],
  ['multiline explicit await transform and terminal >> val capture',
    [
      'fnc plus_one(int x): int { ^x+1; }',
      'pub susp fnc main(): int {',
      '  val pending=task { return 41; };',
      '  ^pending',
      '    |> susp |f| -> await f',
      '    |> plus_one',
      '    >> val result: int;',
      '}'
    ].join('\n'),
    [
      'fnc plus_one(int x): int { ^ x+1; }',
      'pub susp fnc main(): int {',
      '  val pending=task { return 41; };',
      '  ^ pending',
      '    |> susp |f| -> await f',
      '    |> plus_one',
      '    >> val result: int;',
      '}'
    ].join('\n')],
  ['no transform existing semantics', 'fnc f() -> int :: const z: int=5, z+5 end',
   'fnc f() -> int :: const z: int=5, z+5 end'],
  ['ordinary begin', 'pub fnc main(): int { ^ begin let z=5, z+3 end; }',
   'pub fnc main(): int { ^ begin let z=5, z+3 end; }'],
  ['comments and strings', [
     '// ^42 (>let\n',
     '/* ^42 /* (>let */ ^42 */',
     'pub fnc main(): int { val x = "^42 (>let"; ^42; }'
   ].join('\n'), [
     '// ^42 (>let\n',
     '/* ^42 /* (>let */ ^42 */',
     'pub fnc main(): int { val x = "^42 (>let"; ^ 42; }'
   ].join('\n')],
  ['never interpret caret as infix', 'fnc f(): int { return 5 ^ 3; }',
   'fnc f(): int { return 5 ^ 3; }'],
  ['never merge return and XOR', 'fnc f(): int { ^^; ^; }',
   'fnc f(): int { ^^; ^; }'],
  ['never alter namespace and channel operators', 'fnc f(): int { ^ pkg::f(); io >> const x; }',
   'fnc f(): int { ^ pkg::f(); io >> const x; }'],
  ['no rewriting ^ const invalid syntax', 'fnc f(): int { ^ const x = 5; }',
   'fnc f(): int { ^ const x = 5; }'],
  ['do not change return keyword', 'fnc f(): int { return 42; }',
   'fnc f(): int { return 42; }'],
];
for(const [name,input,expected] of canonical){
  test(name,()=>{
    const output=normalize_caret_sequence_spacing(input);
    assert.equal(output,expected);
    assert.equal(normalize_caret_sequence_spacing(output),output,'format twice stable');
  });
}
test('reject malformed comments, strings and oversize input',()=>{
  assert.throws(()=>normalize_caret_sequence_spacing('/* hi'),/Unterminated nested comment/);
  assert.throws(()=>normalize_caret_sequence_spacing('fnc x(): int { ^ "oops; }'),/Unterminated string/);
  assert.throws(()=>normalize_caret_sequence_spacing('x'.repeat(1024*1024+1)),/MiB/);
  assert.throws(()=>normalize_caret_sequence_spacing(null),/string required/);
});

const wasm=new Uint8Array([0,97,115,109,1,0,0,0]);
const wasmDifferent=new Uint8Array([...wasm,0,1,0]);
test('host verifier checks unchanged and rewritten guest byte identity',async()=>{
  assert.ok(WebAssembly.validate(wasm));
  assert.ok(WebAssembly.validate(wasmDifferent));
  const changed=await normalize_verified_caret_sequences('pub fnc main(): int { ^42; }',async()=>wasm);
  assert.equal(changed.formatted,'pub fnc main(): int { ^ 42; }');
  assert.equal(changed.changed,true);
  const stable=await normalize_verified_caret_sequences(changed.formatted,async()=>wasm);
  assert.equal(stable.changed,false);
  await assert.rejects(()=>normalize_verified_caret_sequences('pub fnc main(): int { ^42; }',
    async text=>text.includes('^42')?wasm:wasmDifferent),/guest Wasm changed/);
  await assert.rejects(()=>normalize_verified_caret_sequences('pub fnc main(): int { ^42; }',
    async()=>{throw new Error('E_TYPE');}),/E_TYPE/);
  await assert.rejects(()=>normalize_verified_caret_sequences('pub fnc main(): int { ^42; }',
    async()=>new Uint8Array([...wasm,77])),/valid Wasm module/);
  await assert.rejects(()=>normalize_verified_caret_sequences('pub fnc main(): int { ^42; }',null),/required/);
});

import {mkdtemp,readFile,writeFile,rm} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const attoc=process.env.ATTOC?resolve(process.env.ATTOC):null;
async function compileGuest(source){
  const tmp=await mkdtemp(join(tmpdir(),'atto-caret-verified-'));
  try{
    const file=join(tmp,'input.atto'),dst=join(tmp,'result.wasm');
    await writeFile(file,source,'utf8');
    execFileSync(attoc,[file,'-o',dst],{timeout:15000,stdio:'pipe'});
    return await readFile(dst);
  }finally{await rm(tmp,{force:true,recursive:true});}
}
test('real compiler output identical for caret and nested sequence formatting',
  {skip:!attoc},async()=>{
    for(const src of [
      'pub fnc main(): int { ^42; }',
      'pub fnc main(): int { ^seq let x=5, x+37; }',
      'pub fnc main(): int { ^(>let x=5,(>let y=x+3,y+34)); }',
      'pub fnc main(): int { guard true { ^-1; } ^42; }',
      'pub fnc main(): int { guard false { ^-1; } ^42; }',
      'fnc plus_one(int x): int { ^x+1; } pub susp fnc main(): int { val pending=task { return 41; }; ^pending |> susp |f| -> await f |> plus_one >> val result: int; }',
      'pub fnc main() -> int :: const x: int = 40, x+2 end',
    ]){
      const result=await normalize_verified_caret_sequences(src,compileGuest);
      assert.equal(result.guest_sha256_same,true);
      assert.equal(normalize_caret_sequence_spacing(result.formatted),result.formatted);
    }
});
