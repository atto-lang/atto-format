# PR #92 — one-action-per-line semantic reconciliation

Old head `77d55415643a63b7fc41f65b9f5e855ec0dfdc46`; current main `a078b32321f8aa3ad96400cf2c2b7bb1ed60fa6d`.

Current main already contains newer `format_source_impl`, explicit `format_source_with_mtd`, callable/method signature and lambda layout semantics. The historical full `src/lib.rs` snapshot is not a safe drop-in replacement. Its old CI python scripts are also not reintroduced under the repository's Rust-only policy.

Preserve today's formatter unchanged. Port two lexical safety/oracle tests ensuring semicolons and control keywords inside quoted guest strings remain opaque and idempotence is stable. The original separate `statement_lines.rs` and `braced_control_lines.rs` implementation and 20+ statement-layout cases are intentionally **not claimed complete** by this reconciliation: they require semantic integration into the modern pipeline, exact-head Cargo/MSRV/clippy and compiler parse-format-parse verification before merge. Do not treat a conflict-free branch as implementation of automatic same-line statement splitting.

Review pinning, private-CI policies and interpreter/compiler sync remain separate gates; no cross-org private mirror is authorized.
