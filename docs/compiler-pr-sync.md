# Compiler PR compatibility checkpoint — 2026-10-06

This formatter is intentionally a *syntax-preserving tool*, not evidence that a compiler branch has landed. Keep both legacy `case readch input: val x { ... }` and proposed binding-first `case val x = readch(input) -> { ... }` valid formatter inputs. Never silently rewrite one into the other, or strip `const`, `val`, `let`, or `mut`.

Upstream PRs (all open at inspection): [#398](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/398), [#399](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/399), [#404](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/404), [#405](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/405), [#406](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/406), [#408](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/408), [#409](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/409), [#410](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/410), [#411](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/411), [#412](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/412).

## Formatter regression gates

- Keep `select`/`nb select`/`try select` legacy colon cases byte-semantically stable.
- Accept `do select` and `do nb select` with both braced case styles.
- Preserve binding-first arrow case heads and `mut` exactly; no ownership inference in a formatter.
- Do not transform `return value;` into an implicit result, particularly in no-result arms.
- Do not rewrite `do match` to `match`, or `cb select` to anything: diagnostics belong to the compiler.
- Run `cargo test` and assert idempotence. Re-test under an exact upstream PR head SHA before claiming runtime conformance.

The compiler select PRs form a stack: #405 -> #406 -> #408 -> #412. #404 follows another match branch. Do not bump a stable compiler pin to an unmerged head without explicit feature-branch CI.
