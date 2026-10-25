# Private formatter source mirroring

Private `ores-truffle-oreslang/oreslang-format` source must never be copied to a public or internal repository. A successful source-free Actions probe, repository write permission, or direct GitHub connector access is not authorization to transfer source.

## Deny-by-default authority

`.github/private-ci-approved-repositories.txt` is empty by default and `.github/private-ci-carrier-contracts.json` contains no carriers. Until both contain an exact independently reviewed destination contract, no cross-repository source transfer is authorized.

Every source-bearing transfer must additionally require all of the following:

- source repository identity is exactly `ores-truffle-oreslang/oreslang-format` / numeric repository ID `1406328272`, private and active;
- source `main` is protected with required pull-request review, CODEOWNER review, stale-approval dismissal, at least one independent approval, administrator enforcement, force-push/deletion disabled, and no review bypass;
- the exact source PR head commit and complete root Git tree are pinned immediately before transfer;
- the destination is private, active, has the exact reviewed numeric repository ID, and its final reviewed workflow/test-plan blobs match the source contract;
- the publisher verifies every source path, mode, blob SHA and bounded size before any destination write;
- the carrier executes a reviewed immutable plan without source credentials or outbound network;
- one-use payload and authorization refs are cleaned up after success or failure.

Missing branch-protection visibility or `protected=false` fails closed. There is no formatter source-export fallback that treats a review comment, source-free carrier result, or unprotected source governance PR as equivalent to live source-main protection.

## Review-only carrier candidate

`sonus-auris-test/clients-api-contract` (repository ID `1323599063`) is currently a **review-only** source-free carrier candidate. The formatter plan is intended to bind PR #91, Rust 1.85, the shared type-declaration fixture, focused formatter tests, full offline Cargo tests, clippy, rustdoc, and release binary builds.

This file does not approve Sonus and does not authorize direct connector tree/blob/ref writes. The stacked V3 publisher review is itself non-authorizing and must be independently reviewed, merged to protected source `main`, and byte-pinned before any formatter source leaves the source repository.

## Current blocker

As of 2026-10-25, GitHub reports formatter `main` as `protected: false`. The available GitHub App connection does not expose repository Administration-level branch-protection mutation. An authorized repository administrator must configure and independently verify source-main protection before any source-bearing carrier handoff.


## V3 publisher review requirements

The V3 publisher is intentionally narrower than the compiler publishers. Reviewed formatter plans may execute only `python3` or tightly admitted `cargo` subcommands. Cargo test/clippy/rustdoc/build operations must be offline; manifest/config/registry/index/target-dir/path overrides are rejected. Cargo plans must bind the exact `Cargo.toml` blob.

The publisher must refuse every export unless formatter source `main` is live-protected with the required CODEOWNER/review/admin/no-bypass/no-force-push/no-delete controls. Unprotected source review attestation is not a fallback.
