# Compiler and formatter: same source, independent duties

Oreslang has **one canonical source layout**, but formatting and compilation
are intentionally different admission steps.

- `oresfmt --check path/to/src` checks canonical layout **read-only**. It does
  not establish grammar, import, type, effect, ownership or backend validity.
- The Java/GraalVM launcher `oreslang check file.ores` is the canonical public
  spelling for linked compiler validation (including syntax errors), without
  executing the guest entrypoint. Legacy `--check` remains an alias.
- The stack CLI's `oreslang-stack check` already performs generated-adapter
  drift verification and invokes `oreslang check` for every `.ores` source.
  Its companion `--format-check` option (in the separate stack CLI change)
  then runs `oresfmt --check` on the same source tree.
- `oreslang-stack build --format-check` requires both checks **before**
  packaging, and must not create a source bundle on a formatting failure.
  Other build-profile checks are still owned by the compiler, not oresfmt.

## No same-line executable actions or keyword control blocks

Compatibility input:

```ores
if foo then x(); fi
first(); second();
```

Canonical result:

```ores
if foo then
  x();
fi

first();
second();
```

The formatter **must not** change `x()` into another invocation, repair a
malformed token sequence, or claim invalid code has become valid. Existing
compiler-compatible compact keyword spelling remains parseable; the formatter
normalizes its layout. A missing `fi` or an invalid expression remains a
compiler diagnostic. Semicolons embedded in strings, comments, argument lists
and record literals are not statement separators.

## Executable acceptance chain

For every syntax fixture admitted by the compiler, the release gate is:

```text
original source
  -> oreslang check
  -> oresfmt
  -> oreslang check
  -> oresfmt again (byte-identical)
  -> oreslang-stack build --format-check
```

A compiler rejection stops the chain before formatting. A formatting rejection
stops the chain before packaging. Neither check nor build may rewrite source.
Compact-but-valid source is a compiler concern; canonical physical layout is a
formatter concern.

## Release gate before enabling formatting by default

Run the real Java `oreslang check` command on **both** source and formatted
source, plus real LLVM compiler admission for the backend's supported subset.
Require:
formatter idempotence, comments/literal byte preservation, equivalent linked
compiler admission, and semantic/AST checks wherever available. Formatter
syntax-only tests and mock compiler harnesses cannot substitute for actual
compiler runs. Keep auto-`--write` out of check/build; do not silently repair
syntax or bulk-mutate a checkout.

## Multiline braces and ambiguity boundaries

A recognized executable control opener is placed on its own physical line
even when the closer appears later in the file. A joined `} else {`, `}
catch {` or `} finally {` is separated into distinct lines. Consecutive
executable actions such as `first(); second();` inside the body are split.

Conversely, multiline `infer struct { ... }` values and parenthesized
argument expressions must remain lexically intact; a semicolon inside either
is not an executable statement boundary. An attached typed literal such as
`Point{x: 1}`, or an expression whose first `{` may be a data value,
is deliberately **not** rewritten as a control brace by the lexical pass.
The compiler, not a guessing formatter, is responsible for disambiguation.

The passing bar is **executed**, exact-head `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test` on the
stable/MSRV toolchains, and parse→format→parse checks against actual Java
and LLVM compilers. A GitHub Actions job that records **zero executed steps**
is blocked runner admission, not compiler/test evidence.
