> **AI agent instructions (organization-wide):** Before working on code, issues, or pull requests, read and follow the [organization `AGENTS.md`](https://github.com/atto-lang/.github/blob/main/AGENTS.md), including its **merge-first stealth-development policy**. Honor any repository-local `AGENTS.md`, explicit holds, and GitHub-enforced rules.

# atto-format

Canonical code formatter and formatting rules for AttoLang source code.

## Naming migration and language authority

This repository is maintained at [atto-lang/atto-format](https://github.com/atto-lang/atto-format); use `https://github.com/atto-lang/atto-format.git` for Git remotes and canonical `.atto` source examples. The normative language contract is [atto-spec](https://github.com/atto-lang/atto-spec), with executable guest evidence tracked by [atto-platform-conformance](https://github.com/atto-lang/atto-platform-conformance). Follow current ownership, borrowing, effects, actor, and syntax rules there; do not infer implemented features from a README or unexecuted CI.

## Purpose

Specify deterministic formatting and round-trip parsing behavior.

## Repository responsibilities

- Keep interfaces, schemas, and behavior documented in a backend-neutral form where applicable.
- Use test fixtures and reproducible evidence to distinguish proposed behavior from implemented behavior.
- Record breaking changes, migration notes, and dependencies on other AttoLang components.

## Return-type formatting rule

Named AttoLang callables must explicitly declare a return type, including `-> void`. Explicit `-> infer` is accepted but a concrete type is preferred. The lexical formatter preserves these annotations, does not synthesize a result for omitted annotations, and refuses to normalize malformed unannotated method headers. A verified formatter rewrite still requires the real compiler to accept both programs and emit byte-identical guest artifacts. See [atto-spec](https://github.com/atto-lang/atto-spec/blob/main/language/explicit-callable-return-types-v1.md).

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

AttoLang is the canonical project name; new source, examples and documentation use `.atto` and the `atto-lang` repository organization. This README defines scope, not feature readiness.

**Acceptance:** name the exact source SHA/tree, runtime/toolchain, positive and negative checks and execution receipt before claiming support. Host-only tests and empty CI steps do not prove guest execution.

## Out-of-line constructor header whitespace (target v1)

The supported constructor *header* spellings `constructor (Counter)(int initial)`,
`constructor (Counter) (int initial)`, and `constructor(Counter) (int initial)`
are lexically identical. The bounded `normalize_callable_headers` pass now
canonicalizes **complete** `: self { ... }` out-of-line declarations to
`constructor (Counter)(int initial): self { ... }`. It preserves comments,
quoted text, inline `constructor(args)` class bodies, and malformed/incomplete
headers. Idempotence and lexical regression tests cover all three forms.

This is a source-only formatter change. The verified wrapper
`normalize_verified_callable_headers` continues to require the *real*
compiler to produce valid, **byte-identical** guest Wasm before admitting a
rewrite. No claim of class/struct constructor backend parity or state runtime
readiness follows from the lexical tests. Normative contract:
[atto-spec PR #98](https://github.com/atto-lang/atto-spec/pull/98).
Companion Rust frontend change: [atto-source.rs PR #354](https://github.com/atto-lang/atto-source.rs/pull/354).

## Ownership operation normalization (staged v1)

The limited src/ownership-operations.mjs pass preserves distinct ownership
spellings such as copy!! versus copy! !value, borrow! mut, freeze!, weak!,
pin! and drop!. It canonicalizes a recognized lifetime! binder to lt!
without touching quoted text, templates, comments or ambiguous member
syntax. It is **not** a full AttoLang parser/formatter.

The verified wrapper accepts a caller-provided *real guest compiler* and
refuses a rewrite unless both input variants generate identical WebAssembly.
Source-only tests of a stub compiler callback are contract tests, not real
guest-language evidence.

Run locally:
    node --test tests/ownership-operations.test.mjs

The exact committed JS source and test blobs were run on Node 22:
**9 tests passed, 0 failed**. This does not establish JVM/LLVM/.NET
guest compiler parity or automatic formatting of unsupported grammar.
Related source and language PRs: atto-source.rs#344, atto-spec#92,
atto-borrow-checker.rs#43 and atto-platform-conformance#76.
