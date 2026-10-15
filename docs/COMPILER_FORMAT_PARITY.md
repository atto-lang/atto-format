# Compiler-backed formatter acceptance parity (experimental)

The formatter is not a semantic compiler. It cannot prove that two programs
have identical runtime behavior. This harness offers a **narrower, honest gate**:
the original and formatted source must retain the same acceptance status in
each independently provided compiler, and formatting must be idempotent.

The fixtures in `tests/compiler_parity_cases` are deliberately restricted to
the small scalar intersection of the current Java/GraalVM and LLVM frontends,
based on LLVM's `constant-main.ores` and `forward-call-main.ores` fixtures.
They are not evidence of actor, networking, streams, channels, or import parity.

## Run against real compiler executables

Build the reviewed formatter and both compilers independently. Use their real
command-line invocations (with parse/compile-only options as applicable):

```sh
cargo build --bins
python3 scripts/check_compiler_parity.py \
  --formatter './target/debug/oresfmt' \
  --java-command 'JAVA_COMPILER_COMMAND {source}' \
  --llvm-command '/path/to/oreslang-llvmc {source}' \
  --require-shared-accepted
```

Replace `JAVA_COMPILER_COMMAND` with an actual configured Java compiler
invocation that exits zero for admitted source and nonzero on rejection.
No default Java CLI is assumed; don't treat an `oreslang` interpreter run
as compile-only unless explicitly configured to be so.

The harness creates fresh temporary source files, does **not** execute
generated guest binaries, never invokes a shell, and records only compiler
accept/reject/timeout/unavailable status. It fails if the formatter is not
idempotent, if either compiler changes its acceptance decision after
formatting, or if either backend times out or cannot run. The optional
`--require-shared-accepted` ensures at least one fixture is actually
accepted by both backends rather than reporting all-rejected as a success.

An individual backend rejecting both forms is reported as unsupported, not
as proof of semantic equivalence. Compiler acceptance parity alone is **not**
a runtime, AST-equivalence, ownership, or generated-code equivalence proof.

## CI separation

`python3 -m unittest discover -s scripts -p 'test_compiler_parity.py'`
tests harness behavior with deliberately simulated compiler commands. This
is a **harness self-test only**. Actual Java/LLVM cross-backend admission
requires independently built compiler binaries and the command above; those
are not silently substituted or fetched across private organizations.

All fixtures should eventually be annotated with explicit backend
capability tiers once Java and LLVM offer stable parse-only entrypoints.
The proposed next gate is to compare compiler ASTs under a normalized
representation, not just exit codes, and then add runtime differential
checks for their common executable subset.
