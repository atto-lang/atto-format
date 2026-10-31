# Five-backend semantic AST equivalence gate (opt-in)

The existing scripts/check_compiler_parity.py checks compiler admission and
formatter idempotence. It does not prove that formatting preserves an AST.
Its Python mock tests are not real compiler evidence.

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
  --cases tests/compiler_parity_cases
~~~

These placeholders are not working commands. Until all five compilers provide
reviewed adapters, this real-backend gate is NOT EXECUTED and NOT PASSED.

## Evidence, test scope and safety

- Every .ores file in the cases directory is a positive expected-to-parse
  fixture. Empty corpora, missing backends, malformed JSON (including
  duplicate keys, nonfinite numbers and empty AST envelopes), nonregular
  files, process failures, non-idempotence and AST divergence fail closed.
- The formatter uses --stdout, never --write. Source bytes, process output
  and run time are bounded; process group termination comes from the
  existing check_compiler_parity.run_process implementation.
- Tests in scripts/test_compiler_ast_parity.py are source-free mock harness
  self-tests, NOT real Java/LLVM/WASM/.NET/Forge execution.
- Negative source fixtures require independent tests proving invalid source
  stays rejected. Existing compiler admission checks cover part of this
  contract but are not semantic AST proof.
- Real proof must record exact formatter commit/tree, compiler/adapter SHAs
  and binary identities, fixture manifest/digests, executed steps and runner
  evidence. Zero-step GitHub Actions failures are not compiler test failures.
- Source mirroring is disallowed unless the separate independent private-CI
  approvals in docs/PRIVATE_CI_MIRROR_PREFLIGHT.md are satisfied.

The gate is not yet a required production workflow. Installing the harness
does not establish five-backend parity; reviewed compiler AST adapters and
real executable evidence remain necessary.
