/**
 * Safe adapter: write nothing unless the actual source compiler proves the
 * original and normalized programs emit IDENTICAL, valid Wasm bytes.
 * This does not establish runtime equivalence across other backends.
 */
import {normalize_caret_sequence_spacing} from './caret-sequences.mjs';

export async function normalize_verified_caret_sequences(
  source, compileGuest, {language_version=1}={}
) {
  if(typeof source!=='string' || typeof compileGuest!=='function')
    throw new TypeError('Atto source and real compiler callback required');
  if(language_version!==1 && language_version!==2)
    throw new RangeError('Unsupported Atto language_version; choose 1 or 2');
  // An opt-in strict-v2 caller must provide a compiler callback that accepts
  // an explicit edition; otherwise a legacy-only compiler might attest the
  // wrong grammar and authorize a destructive write.
  if(language_version===2 && compileGuest.length<2)
    throw new TypeError('Version 2 formatting requires a version-aware compiler callback');
  const formatted=normalize_caret_sequence_spacing(source);
  const compile=async text=>{
    const guest=await compileGuest(text, language_version);
    if(!(guest instanceof Uint8Array) || guest.byteLength<8 ||
       guest.byteLength>16*1024*1024 ||
       Buffer.from(guest.subarray(0,8)).toString('hex')!=='0061736d01000000' ||
       !WebAssembly.validate(guest))
      throw new Error('Compiler failed to return a bounded valid Wasm module');
    return Buffer.from(guest);
  };
  const first=await compile(source);
  const second=formatted===source?first:await compile(formatted);
  if(!first.equals(second)) throw new Error('Unsafe caret/sequence formatting: guest Wasm changed');
  return {ok:true,changed:source!==formatted,formatted,guest_sha256_same:true};
}
