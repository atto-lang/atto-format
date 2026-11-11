# Read-only formatter audit against compiler tips

The formatter's normal `cargo test` suite validates formatter behavior, **not**
whether current compiler repositories are formatting successfully. This
separate offline gate enumerates *tracked* `.atto` and legacy `.ores` source
files from local compiler checkouts, executes `oresfmt --stdout` twice for
every file, and checks byte-identical idempotence. It does not rewrite the
compiler worktrees. Deliberately invalid/negative compiler fixtures are
included and may be reported as formatter rejections; do not hide those
failures or count them as successful, compiler-accepted source.

## Snapshot from November 10, 2026

The following GitHub **main commit SHAs** were observed at the start of this
audit. They are a *snapshot*, not a promise to track future tips automatically.
Fetch fresh main heads from an authorized source for the next audit, then
substitute those exact hashes below:

| Backend | Repository | HEAD |
| --- | --- | --- |
| Java/GraalVM | oreslang-source.java | `6d8eb70048eafc9a84c5aebe9c1bb06104f16e76` |
| LLVM | oreslang-source.llvm | `0ee037ca3e5dd9ac6ac1e9cc3d40f19ee6326a9c` |
| .NET | oreslang-source.net | `3f3606cc3ef81dad117047b4cb57609ad1a92672` |
| Wasm | oreslang-source.wasm | `2c6ce5988592080ea47b8877e2b8c1d3df5cb9bd` |
| Forge | oreslang-source.forge | `2acf59d589f8016f879581f17a27df144fa73075` |
| Rust | oreslang-source.rs | `e0b8124c69d486c2bd919fb2d0124ea4eb87159e` |

## Invocation

Use six **authorized, clean, local** compiler checkouts; this script makes no
network requests and never transfers private repositories. Build the formatter
at an independently pinned, reviewed Git commit first.

```sh
cargo build --bins
python3 scripts/audit_compiler_tip_format.py \
  --formatter ./target/debug/oresfmt \
  --require-all \
  --repo java=/work/oreslang-source.java@6d8eb70048eafc9a84c5aebe9c1bb06104f16e76 \
  --repo llvm=/work/oreslang-source.llvm@0ee037ca3e5dd9ac6ac1e9cc3d40f19ee6326a9c \
  --repo net=/work/oreslang-source.net@3f3606cc3ef81dad117047b4cb57609ad1a92672 \
  --repo wasm=/work/oreslang-source.wasm@2c6ce5988592080ea47b8877e2b8c1d3df5cb9bd \
  --repo forge=/work/oreslang-source.forge@2acf59d589f8016f879581f17a27df144fa73075 \
  --repo rs=/work/oreslang-source.rs@e0b8124c69d486c2bd919fb2d0124ea4eb87159e \
  > formatter-tip-audit.json
```

The tool requires each local `HEAD` to match the *full* supplied SHA and
rejects uncommitted tracked changes. It records the checked-out Git tree SHA,
checked source count, changed-source count, an ordered SHA-256 source inventory
digest, and per-file rejection diagnostics. It requires at least one tracked
source per repository, refuses symlinked, binary, oversized or non-UTF-8 source,
and enforces bounded formatter process output/time with source-free process
isolation. It has no secret token, no shell invocation for formatter commands,
and no `--write` path.

Ordinary mode **allows a formatting difference** but flags it in `changed`:
this checks that the CLI can safely and consistently format the compiler
source, not whether every compiler repo follows canonical style already.
Add `--require-clean` to make source-style drift fail the audit. Any
formatter crash/rejection, idempotence defect, empty-source erasure, or
repository mismatch fails the audit in both modes. The inventory is complete
only when `--require-all` is used with all six backends. The optional JS
transpiler may be provided with `--repo js=...`; its presence does not
substitute for a required backend.

## Important limits

- A local HEAD equals the supplied SHA; it does **not** establish that the SHA
  is still the remote default-branch tip. Refresh and separately authenticate
  the source-of-truth main refs before treating the inventory as current.
- Self-tests in `scripts/test_audit_compiler_tip_format.py` use simulated
  formatters and Git worktrees. They are harness tests, **not** compiler-backed
  acceptance or guest/AOT evidence.
- A format-preserving byte-level fixed point is not an AST-equivalence
  guarantee. Run `scripts/check_compiler_parity.py`, the separately reviewed
  five-backend AST adapters, and independently executed backend guest/AOT
  tests as applicable. Unsupported `--analyze` must not be counted as success.
- The auditor records output hashes but cannot authenticate an arbitrary
  formatter command. Pin and record the formatter binary digest and compiler
  build identity in the external run receipt, plus exact executed CI steps and
  independent review. A workflow with `steps=[]` is **NOT_EXECUTED**.
- Do not assume an alternate CI organization's spare minutes authorize
  mirroring private Git trees. Follow the explicit private-CI security policy
  before any such transfer.
