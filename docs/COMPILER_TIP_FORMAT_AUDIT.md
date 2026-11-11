# SHA-pinned formatter audit against current compiler tips

This gate tests the **actual `oresfmt` executable**, not a Python model of
formatting. It audits every tracked `.atto` and `.ores` input in **all five**
independently checked-out compiler trees, rejects stale/dirty repositories,
checks `oresfmt --stdout` twice for byte-level idempotence, and confirms
that **all tracked compiler-repository files**, including manifests,
configuration and imported resources, remain unchanged. The inventory is
bounded to 20,000 tracked files, 8 MiB per file and 64 MiB per repository. Any untracked or
ignored worktree files fail admission; the receipt records physically
verified tracked-file counts and total tracked bytes by backend. It hashes
physical source bytes against immutable Git `HEAD` blob IDs, so `git status`
shortcuts such as `assume-unchanged` or `skip-worktree` cannot forge a clean
source checkout. The JSON receipt records each compiler Git tree and a
SHA-256 digest of the formatter executable. Git subprocesses strip inherited
`GIT_*` control variables and confirm each checkout is the exact repository
root, rather than a nested directory or a redirected foreign worktree. Git replacement
objects are explicitly disabled; a local `.git/refs/replace` entry cannot
substitute a different HEAD tree for the audited immutable commit.

## Compiler main heads observed on 2026-11-10

| Compiler repository | Observed main commit |
| --- | --- |
| `oreslang-source.java` | `6d8eb70048eafc9a84c5aebe9c1bb06104f16e76` |
| `oreslang-source.llvm` | `0ee037ca3e5dd9ac6ac1e9cc3d40f19ee6326a9c` |
| `oreslang-source.wasm` | `2c6ce5988592080ea47b8877e2b8c1d3df5cb9bd` |
| `oreslang-source.net` | `3f3606cc3ef81dad117047b4cb57609ad1a92672` |
| `oreslang-source.forge` | `2acf59d589f8016f879581f17a27df144fa73075` |

These are observation-time **main** heads, not claims about every open feature
PR. Substitute reviewed feature-branch SHA values when auditing a compiler
PR tip. Never call an old pinned source tree the latest without rechecking
the real repository refs.

## Run in an authorized environment with all five repositories checked out

Build the formatter from the **reviewed, pinned formatter Git commit**, then
run the exact-head audit without mutating any compiler repository:

```sh
cargo build --release --bin oresfmt

python3 scripts/audit_compiler_tip_formatter.py \
  --formatter target/release/oresfmt \
  --repo oreslang-source.java=../oreslang-source.java \
  --head oreslang-source.java=6d8eb70048eafc9a84c5aebe9c1bb06104f16e76 \
  --repo oreslang-source.llvm=../oreslang-source.llvm \
  --head oreslang-source.llvm=0ee037ca3e5dd9ac6ac1e9cc3d40f19ee6326a9c \
  --repo oreslang-source.wasm=../oreslang-source.wasm \
  --head oreslang-source.wasm=2c6ce5988592080ea47b8877e2b8c1d3df5cb9bd \
  --repo oreslang-source.net=../oreslang-source.net \
  --head oreslang-source.net=3f3606cc3ef81dad117047b4cb57609ad1a92672 \
  --repo oreslang-source.forge=../oreslang-source.forge \
  --head oreslang-source.forge=2acf59d589f8016f879581f17a27df144fa73075 \
  --report formatter-tip-audit.json
```

The command fails if **any** compiler tree is missing, dirty, at a different
commit, contains an unsafe tracked source path, has zero tracked AttoLang
source files, or if a source is rejected, non-idempotent or modified.
The JSON receipt contains commit IDs, file counts and failures, not source
bytes. Explicit source paths are never sent over the network by the auditor.

## Compiler-negative fixture handling

The five pinned backend trees include intentional compiler-rejection fixtures.
A filename containing `negative` or `reject` is **not** proof that a formatter
should reject that file: many type-checking/runtime rejection fixtures still
have syntactically valid source. Therefore the tip audit never exempts files
based on path spelling. It records formatter exit-1 cases separately in
`rejections[]` with status `requires-compiler-baseline`, and still fails
the release gate until real compiler acceptance evidence is available.
Formatter exit 2 (tool/usage/filesystem error) is not a compiler rejection.
Failure to format the formatter's own first-pass output is an idempotence error.

Physical post-format source reads use bounded POSIX nofollow directory
descriptors; a formatter swapping a leaf or parent directory for a symlink
cannot trick the audit into reading through it. The gate fails closed if that
secure filesystem interface is unavailable.

## What this gate does *not* prove

A passing formatting/idempotence audit **does not show that a compiler accepts
the formatted output**. In particular, unmerged draft PR
[#127](https://github.com/ores-truffle-oreslang/oreslang-format/pull/127)
reported real Java/LLVM admission regressions against the formatter's
`main` baseline. Do not promote the baseline based on an `oresfmt --check`
result alone.

After the compiler-safe formatter fixes are integrated, separately execute
`scripts/check_formatter_acceptance_delta.py` from PR #127 with real,
version-pinned Java and LLVM compiler binaries, and extend the compiler
admission/AST/guest gates to WASM, .NET and Forge when they expose equivalent
source-checking entrypoints. A source rejected before *and* after formatting
is unsupported, **not** proof of language parity. Check rejects-valid,
accept→reject, reject→accept and AST mismatches.

The script is strictly local: it does not fetch private repositories, mirror
their contents into an unrelated organization, use tokens/secrets, dispatch
GitHub Actions, or establish runtime/AOT equivalence. Use only approved
worktrees/runners and separate exact-tree verification for any later CI
transfer.

## Offline regression commands

```sh
cargo test --test cli_compiler_tip_corpus
python3 -m unittest discover -s scripts -p test_audit_compiler_tip_formatter.py -v
```

These local-fixture tests prove the CLI and audit tool behavior **only**.
The five-compiler corpus and real compiler-admission checks must run separately
before asserting end-to-end conformance.
