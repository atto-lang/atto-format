import test from 'node:test';
import assert from 'node:assert/strict';
import {normalize_verified_caret_sequences} from '../src/verified-caret-sequences.mjs';
import {normalize_caret_sequence_spacing} from '../src/caret-sequences.mjs';
import {mkdtemp,readFile,writeFile,rm} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';

const wasm = new Uint8Array([0,97,115,109,1,0,0,0]);
test('the formatter never migrates unmarked comma bodies or :: selectors',()=>{
  for(const source of [
    'fnc f() -> int const x: int = 40, x + 2 end',
    'fnc f() -> int :: const x: int = 40, x + 2 end',
    'fnc f() -> int const x: int = 40; return x + 2; end',
    'fnc f() -> int :: const x: int = seq(40, 42), x end',
  ]){
    assert.equal(normalize_caret_sequence_spacing(source),source);
  }
});
test('unsupported or missing selected version fails closed',async()=>{
  const src='pub fnc main(): int { ^42; }';
  for(const version of [0,3,2026,'2',null]) {
    await assert.rejects(()=>normalize_verified_caret_sequences(
      src,async()=>wasm,{language_version:version}),/Unsupported Atto language_version/);
  }
  await assert.rejects(()=>normalize_verified_caret_sequences(
    src,async()=>wasm,{language_version:2}),/version-aware compiler callback/);
});
test('version selector propagates to both compiler checks',async()=>{
  const versions=[];
  const source='pub fnc main(): int { ^42; }';
  const value=await normalize_verified_caret_sequences(source,
    async(text,version)=>{versions.push(version);return wasm;},{language_version:2});
  assert.equal(value.changed,true);
  assert.deepEqual(versions,[2,2]);
  assert.equal(value.formatted,'pub fnc main(): int { ^ 42; }');
});

const attoc=process.env.ATTOC ? resolve(process.env.ATTOC) : null;
const tool=resolve('tools/atto-format-caret.mjs');
test('real compiler and formatter agree on explicit language versions',
  {skip:!attoc},async()=>{
  const directory=await mkdtemp(join(tmpdir(),'atto-format-version-'));
  try{
    for(const version of [1,2]){
      const source='fnc f() -> int :: const x: int = 40, x + 2 end pub fnc main(): int { ^f(); }';
      const path=join(directory,`v${version}.atto`);
      await writeFile(path,source,'utf8');
      const check=spawnSync(process.execPath,[tool,'--check',`--language-version=${version}`,path],
        {encoding:'utf8',env:{...process.env,ATTOC:attoc}});
      assert.equal(check.status,1,`expected noncanonical caret v${version}`);
      const write=spawnSync(process.execPath,[tool,'--write',`--language-version=${version}`,path],
        {encoding:'utf8',env:{...process.env,ATTOC:attoc}});
      assert.equal(write.status,0,`versioned write: ${write.stderr}`);
      assert.equal((await readFile(path,'utf8')).includes('^ f();'),true);
      const stable=spawnSync(process.execPath,[tool,'--check',`--language-version=${version}`,path],
        {encoding:'utf8',env:{...process.env,ATTOC:attoc}});
      assert.equal(stable.status,0,`versioned formatter idempotence: ${stable.stderr}`);
    }
    const source='fnc f() -> int const x: int = 40, x + 2 end pub fnc main(): int { ^f(); }';
    const path=join(directory,'unmarked.atto');
    await writeFile(path,source);
    const forbidden=spawnSync(process.execPath,[tool,'--write','--language-version=2',path],
      {encoding:'utf8',env:{...process.env,ATTOC:attoc}});
    assert.notEqual(forbidden.status,0,'v2 must reject legacy-only source');
    assert.equal(await readFile(path,'utf8'),source,'failed v2 migration cannot change source');
    const legacy=spawnSync(process.execPath,[tool,'--write','--language-version=1',path],
      {encoding:'utf8',env:{...process.env,ATTOC:attoc}});
    assert.equal(legacy.status,0,`v1 must retain source acceptance: ${legacy.stderr}`);
    assert.equal((await readFile(path,'utf8')).includes('::'),false,'formatter cannot silently insert ::');
  }finally{await rm(directory,{recursive:true,force:true});}
});
