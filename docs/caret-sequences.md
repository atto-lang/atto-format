# Conservative caret and scoped sequence formatting

Status: **draft / opt-in**, scoped to language spec atto-spec #35 and Rust compiler atto-source.rs #185. This does not enable the breaking `::` callable-body migration or claim full compiler parity.

The callable body forms are deliberately distinct:

```atto
define class Foo as
    rec1() -> int
        const z: int = 5;
        ^ z + 3;
    end

    rec2() -> int ::
        const z: int = 5,
        z + 5
    end
end
```

Do not turn one form into the other by replacing commas with semicolons. The comma body's implicit return, early exits, ownership and effects must be checked with a language version and explicit migration policy before any conversion.

## Contextual last-value return

The syntax `return ^ (first(), second(), last())` returns the last expression after evaluating earlier expressions once, using the same checked lexical sequence lowering as `seq` and `(> ... )`. It differs from `return (first(), second(), last())`, which constructs a tuple, and from a leading standalone `^ (a, b)`, which returns a tuple. `return ^ ()` is invalid (`E_EMPTY_SEQUENCE`).

This formatter only inserts spacing in proven lexical contexts:

```atto
// Before
pub fnc main(): int { return^(40,42); }
// After
pub fnc main(): int { return ^ (40,42); }
```

It does **not** rewrite tuple commas into sequence commas or make `return x, y;` an alias. Formatter verification must use a compiler that actually implements this contextual form; an older compiler must reject rather than silently canonicalize it. Tuple-valued guest execution and backend parity are separate unfinished gates.

## What the opt-in formatter changes

```atto
// Before
pub fnc main(): int { ^(>let x=5,(>let y=x+3,y)); }

// After
pub fnc main(): int { ^ (> let x=5,(> let y=x+3,y)); }
```

Only whitespace is added at recognized statement-position `^` return shorthands or contiguous `(>` sequence openers. It preserves comments, strings (including nested block comments), `^^`, `^=`, `::`, `>>`, `return`, `seq`, `begin/end`, all commas, terminators, operators, type names, ownership qualifiers and source order. It does not interpret ordinary `(a,b)` tuples or change namespace lookup.

This helper is not a full parser and does **not** declare invalid syntax valid. The *verified* wrapper compiles source **before and after** and rejects invalid/unsupported source, non-Wasm outputs, byte changes, resource oversize, or unavailable compilers. Host-only formatting tests are not guest execution proof.

## Opt-in CLI

```sh
ATTOC=/path/to/verified/attoc node tools/atto-format-caret.mjs --check file.atto
ATTOC=/path/to/verified/attoc node tools/atto-format-caret.mjs --write file.atto
```

`--check` does not write and signals noncanonical source; `--write` only performs an atomic guarded write after identical compiled guest bytes were proven, and refuses symlink/non-regular files, oversized source, and files that changed since verification. The CLI never checks out private cross-organization source code and must use a compiler trusted for that language version.

## Testing and evidence gate

Run `node --test tests/caret-sequences.test.mjs` for host tests. With `ATTOC` set, tests additionally require the actual Rust compiler to compile original and normalized positive programs to byte-identical Wasm. Other backends and non-scalar sequences remain unverified. Compatibility with formatter PR #3's stricter Wasm verifier and PR #4's `susp` normalization requires a later semantic integration check; this independent module intentionally avoids overwriting either PR's modified files.

No generic source-to-source `::` migration is implemented here. Preserve previous `-> Type ... end` semantics until the versioned language gate in atto-spec #35 is resolved.
