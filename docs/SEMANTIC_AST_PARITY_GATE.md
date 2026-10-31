# Five-backend semantic AST equivalence gate (opt-in)

The existing scripts/check_compiler_parity.py checks compiler admission and
formatter idempotence. It does not prove that formatting preserves an AST.
Its Python mock tests are not real compiler evidence.

The versioned tests/formatter_fixture_manifest_v1.json pins each positive
and negative fixture by name and SHA-256. A changed, added, deleted or
unlisted .ores file blocks verification until the reviewed manifest is updated.

This opt-in gate compares original and formatted semantic ASTs for every
backend, and then compares the canonical semantic AST across Java, LLVM,
WASM, .NET and Forge. All five are required: missing commands, unsupported
positive fixtures, changed ASTs, rejected source and malformed JSON fail.

## Required compiler adapter contract

Each backend must expose a separately reviewed, independently executable
compiler/parser adapter. The command receives a {source} path and emits
only UTF-8 JSON to stdout:

~~~json
{
  "schema": "oreslang-semantic-ast-v1",
  "ast": {
    "kind": "module",
    "declarations": []
  }
}
~~~

The sample illustrates the envelope, not a complete semantic AST schema.
The adapters must agree on the complete canonical schema before real parity
claims. Include every relevant declaration, field, mutability/ownership
qualifier, initial value, generic argument, callable effect, exception
behavior, symbol identity, actor-state boundary, match/select arm and
evaluation-order field. Omit only proven nonsemantic attributes, such as
source ranges and comments. The harness never strips AST attributes,
and may fail closed on backend-specific representations.

Do not emit a fabricated constant AST, disguise parser errors as ASTs, or
use a compiler-independent lexical approximation. Review and pin the
adapter commands to actual backend compiler revisions: the harness cannot
authenticate commands merely because they produce matching JSON.
JSON object key ordering is ignored; semantic fields and array ordering
are preserved.

Example invocation (replace placeholders with approved real adapters):

~~~sh
python3 scripts/check_compiler_ast_parity.py \
  --formatter './target/release/oresfmt' \
  --ast-command 'java=<approved-java-adapter> {source}' \
  --ast-command 'llvm=<approved-llvm-adapter> {source}' \
  --ast-command 'wasm=<approved-wasm-adapter> {source}' \
  --ast-command 'net=<approved-net-adapter> {source}' \
  --ast-command 'forge=<approved-forge-adapter> {source}' \
  --cases tests/compiler_parity_cases \
  --negative-cases tests/compiler_negative_cases \
  --fixture-manifest tests/formatter_fixture_manifest_v1.json
~~~

These placeholders are not working commands. Until all five compilers provide
reviewed adapters, this real-backend gate is NOT EXECUTED and NOT PASSED.

## Evidence, test scope and safety

- Positive fixtures must be accepted by all five compilers, and their
  original and formatted semantic ASTs must agree both within and between
  backends. A negative fixture must be rejected by all five compilers on
  its original input. The formatter must either reject that invalid input,
  or produce an idempotent formatted result that all five also reject.
  A formatter syntax rejection is reported separately from a successful
  format/parse rejection-preservation check.
- The manifest must enumerate exactly the positive and negative .ores files.
  Invalid digests, duplicates, missing/extra files, missing negative cases,
  symlinks, oversized inputs, compiler errors and malformed ASTs fail closed.
  A manifest hash is included in the JSON report, but does not authenticate
  the compiler binaries or review approval.
- The formatter uses --stdout, never --write. Source bytes, process output
  and run time are bounded; process group termination comes from the
  existing check_compiler_parity.run_process implementation.
- Tests in scripts/test_compiler_ast_parity.py are source-free mock harness
  self-tests, NOT real Java/LLVM/WASM/.NET/Forge execution.
- The included negative corpus tests a forbidden callable body delimiter,
  an unclosed callable, and assignment in an if-condition. These cases are
  validation targets, not yet proven rejected by five real compilers.
  Additional negative cases must be pinned and reviewed before promotions.
- Real proof must record exact formatter commit/tree, compiler/adapter SHAs
  and binary identities, fixture manifest/digests, executed steps and runner
  evidence. Zero-step GitHub Actions failures are not compiler test failures.
- Source mirroring is disallowed unless the separate independent private-CI
  approvals in docs/PRIVATE_CI_MIRROR_PREFLIGHT.md are satisfied.

The gate is not yet a required production workflow. Installing the harness
does not establish five-backend parity; reviewed compiler AST adapters and
real executable evidence remain necessary.
