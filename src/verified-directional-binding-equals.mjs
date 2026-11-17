/**
 * Fail-closed executable admission gate for the optional directional '='.
 *
 * The caller supplies the real Atto compiler. The lexical normalizer alone
 * is not a proof that the source has a valid Readable<T> / Future<T> ABI.
 * In particular, a syntax-only stdin program still fails this gate.
 */
import { normalize_directional_binding_equals } from './directional-binding-equals.mjs';

export async function normalize_verified_directional_binding_equals(source, compileGuest) {
  if (typeof source !== 'string' || typeof compileGuest !== 'function')
    throw new TypeError('Atto source and a real compiler callback are required');
  if (Buffer.byteLength(source, 'utf8') > 1024 * 1024)
    throw new RangeError('Atto source exceeds verified formatter size limit');

  const formatted = normalize_directional_binding_equals(source);
  async function readGuest(text) {
    const bytes = await compileGuest(text);
    if (!(bytes instanceof Uint8Array) || bytes.byteLength < 8 ||
        Buffer.from(bytes.subarray(0, 8)).toString('hex') !== '0061736d01000000')
      throw new Error('Compiler must produce a valid guest Wasm header');
    return Buffer.from(bytes);
  }
  const before = await readGuest(source);
  const after = formatted === source ? before : await readGuest(formatted);
  if (!before.equals(after)) {
    throw new Error('Unsafe directional binding normalization: guest Wasm changed');
  }
  return { ok: true, changed: formatted !== source, formatted, guest_sha256_same: true };
}
