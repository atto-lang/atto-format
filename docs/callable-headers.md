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
