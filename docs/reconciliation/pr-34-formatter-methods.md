# Reconciliation: formatter PR #34, instance method spelling

Historical PR head `758a6c4bb1443a96d1eb027da8a9d07bf8793bba`; current main `4cca7397655262302f5b194f6e367c3ec92792dc`.

Current `src/lib.rs` already provides the more complete direct class/actor member-scope `fnc` canonicalizer and handles newer lambda, brace-less, comment/literal and one-action-per-line syntax. The historical 63K-line-snapshot formatter must NOT overwrite the 146K current source. Historical `keyword_blocks.rs` also expects semicolons before `then`, whereas main's newer grammar deliberately removes them. Preserve main tests/formatter and the newer grammar.

Forward-port two focused independent Rust tests of canonical instance vs static/module method handling and literal immunity. Full historical `method_spelling.rs` remains in Git ancestry for targeted future adaptation, not a passing current test claim. Require exact-head cargo test, fmt, clippy and compiler parse-format-parse parity before merge.
