# Exact-head GitHub Actions execution receipt

Use `scripts/verify_ci_execution.py` **after** the private source repository's
own GitHub Actions workflow obtains a runner and completes. The command checks
authenticated, read-only GitHub metadata; it does not clone source, transfer Git
objects, copy credentials to another organization, or register runners.

```sh
GH_TOKEN=<metadata-read-only-token> python3 scripts/verify_ci_execution.py \
  --repo ores-truffle-oreslang/oreslang-format \
  --repo-id 1406328272 \
  --pr-number 92 \
  --head <reviewed-exact-40-character-commit-sha> \
  --tree <reviewed-exact-40-character-root-tree-sha> \
  --run-id <actual-source-repository-actions-run-id>
```

A successful receipt requires:

- An active, private source repository with the exact expected numeric ID.
- The live, open PR's current head SHA and source/base repository IDs matching
  the reviewed private source repository. Older passing commits are rejected.
  The PR head is re-read **after** all CI metadata to detect a branch update
  during receipt collection. This is a defensive recheck, **not** an atomic
  lock: the release/merge gate must independently verify the live head again.
- The `pull_request` workflow run must originate from the same private source
  repository and current PR branch, use `.github/workflows/ci.yml`, and list
  that same PR/head SHA in its GitHub metadata. Other workflows and push-only
  runs cannot satisfy the receipt.
- GitHub's immutable commit object pointing to the expected root Git tree.
- The completed successful workflow run belonging to that repository and commit.
- Completed, successful `test` and `msrv` jobs on the current run attempt, each
  with a nonzero runner ID, nonempty runner name, and successful executed steps.
- Observed step labels for required Rust test, lint, format, build and MSRV
  commands. Missing jobs, partial/paginated job responses, wrong-tree results,
  zero-step conclusions and mixed attempts are denied.

A receipt is **metadata evidence**, not an independent test runner, proof that
step labels faithfully reflect commands, proof of compiler semantic equivalence,
or permission to release. Inspect logs and artifacts separately. Run the real
Java/GraalVM and supported LLVM differential tests and stack CLI gates; keep
the formatter PR in draft until those tests execute and pass.

The reader deliberately rejects redirects and ambient HTTP(S) proxies to avoid
sending the metadata credential to an unreviewed endpoint. If the source repo
cannot allocate GitHub-hosted runners, use an approved secure runner **at the
source** or restore the organization's Actions capacity. Never infer test success
from a workflow with zero steps.

A prior public-mirror exposure remains tracked in
[security issue #70](https://github.com/ores-truffle-oreslang/oreslang-format/issues/70).
Neither this receipt nor available capacity at another private organization
authorizes a cross-organization source mirror. An exact source/destination
review, independent security approval, and per-write checks are still required.

Offline tests, with **no** GitHub API calls:

```sh
PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_ci_execution.py' -v
```

Dedicated source-runner recovery blocker:
[issue #94](https://github.com/ores-truffle-oreslang/oreslang-format/issues/94).
