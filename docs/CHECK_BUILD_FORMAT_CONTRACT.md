# Compiler and formatter: same source, independent duties

Oreslang has **one canonical source layout**, but formatting and compilation
are intentionally different admission steps.

- `oresfmt --check path/to/src` checks canonical layout **read-only**. It does
  not establish grammar, import, type, effect, ownership or backend validity.
- The Java/GraalVM launcher `oreslang --check file.ores` runs linked compiler
  validation (including syntax errors), without executing the guest entrypoint.
- The stack CLI's `oreslang-stack check` already performs generated-adapter
  drift verification and compiler `--check` for every `.ores` source.
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

## Release gate before enabling formatting by default

Run the real Java `--check` command on **both** source and formatted source,
plus real LLVM compiler admission for the backend's supported subset. Require:
formatter idempotence, comments/literal byte preservation, equivalent linked
compiler admission, and semantic/AST checks wherever available. Formatter
syntax-only tests and mock compiler harnesses cannot substitute for actual
compiler runs. Keep auto-`--write` out of check/build; do not silently repair
syntax or bulk-mutate a checkout.
