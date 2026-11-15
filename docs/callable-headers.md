# Callable header normalization (bounded reference implementation)

This repository currently has no complete AST formatter. The standalone ES module
`src/callable-headers.mjs` canonicalizes **only recognized method declaration headers**
without changing body families. It is a narrow reference implementation for
[atto-spec #19](https://github.com/atto-lang/atto-spec/pull/19), not proof of
compiler, class-receiver linker, or guest Wasm support.

Rules: within recognized `define class/actor ... as ... end`, remove
redundant instance-method `fnc` and `method` prefixes, preserving `static fnc`.
Canonicalize out-of-line `mtd (Type self)` to `mtd (self: Type)`
and `mtd (Type _)` to `mtd (Type)`. Keep `(mut self: Type)` explicit.
Never mutate comments, strings, unrecognized headers or top-level `fnc`.
Never convert brace bodies to Erlang comma/`end` bodies or change return semantics.

Run `node --test tests/callable-headers.test.mjs`.
**Limitations:** only simple ASCII identifier/type spellings and simple
return-type headers are normalized. Complex generic receivers, nuanced
nested end-delimited grammar, source spans and parser/typechecker integration
must be implemented in the full AST formatter; fail unchanged rather than guess.

## Verified host integration (opt-in)

The `src/verified-callable-headers.mjs` adapter accepts a compiler callback that
returns the **actual executable guest Wasm** for an AttoLang program. It compiles
the original and normalized source and accepts a changed header **only if both
compiler outputs are byte-identical**. Invalid programs, unsupported constructs,
truncated outputs and semantic changes fail closed.

`tools/atto-format-callable.mjs` is an opt-in host CLI:

```bash
ATTOC=/path/to/native/attoc node tools/atto-format-callable.mjs --check program.atto
ATTOC=/path/to/native/attoc node tools/atto-format-callable.mjs --write program.atto
```

`--check` does not mutate input and exits 1 for noncanonical headers.
`--write` only mutates a regular, non-symlink source file after both native
compilations succeed and guest Wasm bytes match. The write is atomic with an
optimistic source-content recheck; unsupported syntax leaves the original
untouched. Without either switch, the verified output goes to stdout.

Unlike a full general-purpose AST formatter this command *does not attempt*
to parse or alter unsupported generic/advanced syntax, whitespace layouts,
control flow, nested expressions or arbitrary semantics. It is restricted to
known method/receiver headers; the full AST formatter still requires its
own parser integration and cross-backend formatting acceptance tests.

`node --test tests/*.test.mjs` runs standalone golden and negative tests.
When `ATTOC` points to the real compiler, it additionally verifies
two executable class receiver fixtures (including legacy inline method
prefixes), idempotence and the atomic CLI's fail-closed operation.
Real compiler comparisons must execute against an exact-source SHA; Node
mock test success alone does not count as semantic equivalence.
