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

Audit follow-up atto-format#14 fixes a previously quadratic formatter
path on 60,000 repeated lifetime binders and adds six stress tests. Both
the lexical context scan and output edit assembly now use bounded linear
work; the 1 MiB source admission cap is unchanged.

Locally verified implementation Git blob:
bb98ca440b418521b38fcb835020d51c105fcf63

Locally verified new stress-test blob:
5bde556210c2ec790f038cd55881aada1d229a89

The Node 22 ownership suite passed **16 tests, 0 failed** locally using
the exact implementation and stress-test blobs; the existing baseline test
fixtures also exercised source/guest binary equivalence. A CI failure before
test execution is **not** a passed runner receipt. This does not establish JVM/LLVM/.NET
guest compiler parity or automatic formatting of unsupported grammar.
Related source and language PRs: atto-source.rs#344, atto-spec#92,
atto-borrow-checker.rs#43 and atto-platform-conformance#76.
