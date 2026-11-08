import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const binary = await readFile('target/wasm32-unknown-unknown/release/oreslang_format.wasm');
assert.ok(binary.length > 1000 && binary.length <= 1024 * 1024,
  'Wasm binary must fit 1 MiB Pages allowlist');
assert.equal(WebAssembly.validate(binary), true);
const module = await WebAssembly.compile(binary);
assert.deepEqual(WebAssembly.Module.imports(module), [], 'formatter may not import any host capability');
const { exports: w } = await WebAssembly.instantiate(module, {});
for (const key of ['memory','oresfmt_alloc','oresfmt_free','oresfmt_format'])
  assert.ok(w[key], 'missing formatter export: '+key);
const decoder = new TextDecoder('utf-8', {fatal:true}), encoder = new TextEncoder();
function format(source, max=262144) {
  const input=encoder.encode(source), inputPtr=w.oresfmt_alloc(input.length), outputPtr=w.oresfmt_alloc(max);
  assert.ok(inputPtr > 0 && outputPtr > 0);
  try {
    new Uint8Array(w.memory.buffer, inputPtr, input.length).set(input);
    const code=w.oresfmt_format(inputPtr,input.length,outputPtr,max);
    assert.notEqual(code,-2147483648,'formatter overflow/invalid input');
    const length=code<0?-code-1:code;
    const text=decoder.decode(new Uint8Array(w.memory.buffer,outputPtr,length));
    return {ok:code>=0,text};
  } finally {
    w.oresfmt_free(inputPtr,input.length);
    w.oresfmt_free(outputPtr,max);
  }
}
const cases=[
  'fnc main() -> int {\nreturn 42;\n}\n',
  'fnc fib(n: int) -> int {\nif n < 2 then\nreturn n;\nelse\nreturn fib(n-1)+fib(n-2);\nfi\n}\n',
  'define class Foo as\nfinal int foo: 5;\nend\n',
  '// do not rewrite "if"\nfnc main() -> int {return 42;}\n',
  'fnc main() -> int {\n  return 42;\n}\n',
];
for (const source of cases) {
  const a=format(source);
  assert.ok(a.ok, 'canonical formatter rejected accepted sample: '+JSON.stringify(a));
  const b=format(a.text);
  assert.deepEqual(b, a, 'canonical formatter not idempotent');
}
const paddedReturn = format('pub fnc main() -> int {\n  val x = 10;\n  x = 20;\n  return   x;\n}\n');
assert.equal(paddedReturn.ok, true, 'Wasm formatter must accept a padded return');
assert.equal(paddedReturn.text, 'pub fnc main() -> int {\n  val x = 10;\n  x = 20;\n  return x;\n}\n');
assert.deepEqual(format(paddedReturn.text), paddedReturn, 'Wasm padded return must be idempotent');
const bareReturn = format('fnc demo() -> void {\nreturn\t   ;\n}\n');
assert.equal(bareReturn.ok, true, 'Wasm formatter rejects padded bare return');
assert.equal(bareReturn.text, 'fnc demo() -> void {\n  return;\n}\n');
assert.deepEqual(format(bareReturn.text), bareReturn, 'Wasm bare return is not idempotent');
const opaqueText = format('fnc demo() -> string {\nreturn   "return   text; //";\n}\n');
assert.equal(opaqueText.ok, true, 'Wasm formatter rejects quoted return');
assert.equal(opaqueText.text, 'fnc demo() -> string {\n  return "return   text; //";\n}\n');
assert.deepEqual(format(opaqueText.text), opaqueText, 'Wasm quoted return is not idempotent');
const unicode=format('// 🚀 日本語\nfnc main() -> int { return 42; }\n');
assert.ok(unicode.ok && unicode.text.includes('🚀 日本語'));
const invalid=format('fnc identity(int x) -> int do return x; end\n');
assert.equal(invalid.ok,false,'canonical formatter rejects retired callable-header do');
assert.ok(invalid.text.length > 0, 'formatter diagnostics are supplied');
assert.equal(w.oresfmt_format(0,1,0,2),-2147483648);
console.log('PASS: real formatter Wasm, canonical idempotence, unicode, rejection and no imports');
