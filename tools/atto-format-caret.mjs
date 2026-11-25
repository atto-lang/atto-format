#!/usr/bin/env node
/** A verified opt-in caret/sequence spacing formatter. No code is rewritten until the actual
 * Rust compiler proves the before/after guest Wasm binaries identical.
 * This deliberately does NOT migrate :: body modes, change return semantics, or claim full AST formatting.
 */
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile, lstat, rename, chmod } from 'node:fs/promises';
import { resolve, dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import { normalize_verified_caret_sequences } from '../src/verified-caret-sequences.mjs';

const args = process.argv.slice(2);
const flag = args.filter(arg=>arg.startsWith('--'));
const paths = args.filter(arg=>!arg.startsWith('--'));
const versions = flag.filter(x=>x.startsWith('--language-version='));
const language_version=versions.length===1 &&
  /^(1|2)$/.test(versions[0].slice('--language-version='.length))
    ? Number(versions[0].slice('--language-version='.length)) : 1;
if(paths.length!==1 || versions.length>1 || versions.some(x=>!/^--language-version=(1|2)$/.test(x)) ||
   flag.some(x=>!['--check','--write'].includes(x) && !x.startsWith('--language-version=')) ||
   (flag.includes('--check') && flag.includes('--write')) || !process.env.ATTOC) {
  console.error('usage: ATTOC=/path/to/attoc node tools/atto-format-caret.mjs [--check|--write] [--language-version=1|2] file.atto');
  process.exit(2);
}
const filename=resolve(paths[0]);
const st=await lstat(filename);
if(!st.isFile() || st.isSymbolicLink())
  throw new Error('Refusing non-regular or symbolic-link source file');
if(st.size>1024*1024) throw new RangeError('Source too large');
const original=await readFile(filename);
const decoder=new TextDecoder('utf-8',{fatal:true});
const source=decoder.decode(original);
async function compileGuest(input, selectedVersion) {
  if(selectedVersion!==language_version)
    throw new Error('formatter/compiler language version mismatch');
  const dir=await mkdtemp(join(tmpdir(),'atto-verified-format-'));
  try {
    const src=join(dir,'input.atto'),dst=join(dir,'guest.wasm');
    await writeFile(src,input,'utf8');
    execFileSync(process.env.ATTOC,['--language-version',String(selectedVersion),src,'-o',dst],{timeout:15000,stdio:['ignore','pipe','pipe']});
    return await readFile(dst);
  } finally { await rm(dir,{recursive:true,force:true}); }
}
const result=await normalize_verified_caret_sequences(source,compileGuest,{language_version});
if(flag.includes('--check')) {
  if(result.changed) {
    console.error('Noncanonical caret/sequence spacing; compiler-verified rewrite available');
    process.exitCode=1;
  }
} else if(flag.includes('--write') && result.changed) {
  const current=await readFile(filename);
  if(!current.equals(original)) throw new Error('Source changed during formatter verification; refusing write');
  const temp=join(dirname(filename),'.atto-format-'+randomUUID());
  try {
    await writeFile(temp,result.formatted,{flag:'wx',mode:st.mode&0o777});
    await chmod(temp,st.mode&0o777);
    await rename(temp,filename);
  } finally { await rm(temp,{force:true}); }
  console.log('Changed: '+filename);
} else if(!flag.includes('--write')) {
  process.stdout.write(result.formatted);
}
