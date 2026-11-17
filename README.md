# atto-format

Canonical code formatter and formatting rules for AttoLang source code.

## Purpose

Specify deterministic formatting and round-trip parsing behavior.

## Repository responsibilities

- Keep interfaces, schemas, and behavior documented in a backend-neutral form where applicable.
- Use test fixtures and reproducible evidence to distinguish proposed behavior from implemented behavior.
- Record breaking changes, migration notes, and dependencies on other AttoLang components.

## Validation

Validation should rely on checked-in tests and exact-commit CI results. When reporting parity, distinguish **passed**, **failed**, **unsupported**, and **not executed**. Avoid inferring guest-runtime support from host-only tests.

## Status and development

This README records the intended role of this repository; it is not a claim of feature completeness. Consult the source tree and workflow files for actual implementation and verified build commands. Contributions should preserve compatibility with the canonical language contract.

## Ecosystem

- [AttoLang](https://github.com/atto-lang)
- [Language specification](https://github.com/atto-lang/atto-spec)
- [Cross-platform conformance](https://github.com/atto-lang/atto-platform-conformance)

## Repository ownership and contribution routing

**Belongs here:** Deterministic AttoLang formatting rules, parser-compatible token/layout preservation, formatting golden fixtures and idempotence tests.

**Belongs elsewhere:** Parser/typechecker semantics belong in atto-source.* and atto-spec; VS Code/Sublime editor packaging belongs in atto-vscode/atto-sublime.

**Security and portability boundary:** Formatter may normalize whitespace but must not change tokens, ownership qualifiers, operators or program meaning. Require parse-roundtrip, negative and idempotence tests.

AttoLang is the public name; preserve existing Oreslang/.ores compatibility and use canonical .atto for new portable library source. This README defines scope, not feature readiness.

**Acceptance:** name the exact source SHA/tree, runtime/toolchain, positive and negative checks and execution receipt before claiming support. Host-only tests and empty CI steps do not prove guest execution.


## Directional `>> =` binding canonicalization (candidate)

`src/directional-binding-equals.mjs` exports
`normalize_directional_binding_equals(source)`, a conservative lexical
normalizer for optional, whitespace-separated `=` after an authorized
directional `>>` binding:

```atto
nb stdio.in >> = Future<Result<Option<string>, IoError>> x
    |> |f| -> (await f)?
    >> = val Option<string> line;
```

The canonical target removes both redundant markers while preserving
`Result`, `Option`, ownership, suspension and evaluation order.
Contiguous `>>=`, arithmetic operations, comments, string literals,
nested-generic `>>` closers, and nonbinding assignment are not rewritten.
Backtick templates are rejected until a full interpolation-aware lexer exists
rather than risking an edit inside embedded source-like text.
This lexical pass is not a compiler or a static capability proof.

`src/verified-directional-binding-equals.mjs` exports
`normalize_verified_directional_binding_equals(source, compileGuest)`.
It returns a usable result **only if a real compiler returns identical
structurally valid guest Wasm bytes for the original and canonical sources**,
using `WebAssembly.validate` instead of trusting the magic header alone. Syntax-only
stdin/Future continuations are not yet guest-executable and therefore must
fail this admission gate until the ABI is implemented. The included
Node-only golden tests do not replace this compiler gate.

Language contract: atto-spec PR #32; staged compiler: atto-source.rs PR #206.
