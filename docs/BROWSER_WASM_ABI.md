# Browser formatter Wasm ABI (experimental)
This is the **canonical** \`oreslang_format::format_source\` Rust library compiled
for \`wasm32-unknown-unknown\`. Browser source never travels to a server.

- \`memory\`: exported WebAssembly linear memory
- \`oresfmt_alloc(length)\`: allocates 1–262144 bytes; returns pointer or 0
- \`oresfmt_free(pointer,length)\`: frees exactly that allocation
- \`oresfmt_format(src_ptr,src_len,dst_ptr,dst_cap)\`:
  - requires 1–65536 source bytes, 1–262144 destination bytes
  - returns 0 or positive UTF-8 length on success
  - returns \`-(diagnostic_bytes+1)\` on formatter error and writes UTF-8 diagnostic
  - returns \`i32::MIN\` on invalid arguments or size overflow
  - never edits the input buffer; no FS, network, WASI or JS imports

This ABI offers formatting only; it does not prove source is valid for the
AttoLang browser compiler. Negative formatter diagnostics must not replace
compiler errors. Browser consumers should run the module in a dedicated worker
with a deadline and enforce the approved binary SHA-256 before instantiation.

Reproducible build:
\`rustup target add wasm32-unknown-unknown && RUSTFLAGS='-C opt-level=z -C panic=abort' cargo build --release --lib --target wasm32-unknown-unknown --offline\`
then \`node tests/browser-wasm.mjs\`.
