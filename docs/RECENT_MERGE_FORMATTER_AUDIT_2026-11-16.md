# Formatter drift audit — merged language changes, November 11–16, 2026

Scope: targeted review of merged grammar, source-lexer, CLI, and source-safety PRs in ores-truffle-oreslang. This is not exhaustive nine-backend compiler or guest parity.

| Merged PR | Resulting formatter responsibility |
| --- | --- |
| [format #141](https://github.com/ores-truffle-oreslang/oreslang-format/pull/141), Nov 13 | Retain .atto and legacy .ores discovery, guarded end-delimited callable bodies. |
| [format #142](https://github.com/ores-truffle-oreslang/oreslang-format/pull/142), Nov 13 | Preserve actor pipe, symbolic actor pipe, and functional pipeline punctuation as distinct source. |
| [LLVM #1776](https://github.com/ores-truffle-oreslang/oreslang-source.llvm/pull/1776), Nov 15 | Keep 2–16-stage actor pipelines ordered. LLVM's merged admission model is not full compiled source pipeline execution. |
| [.NET #510](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/510), Nov 14 | Preserve bounded typed actor-pipe graph source forms, without claiming .NET actor guest/runtime parity. |
| [Java #2798](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/2798), Nov 14 | Keep inline functional lambda pipeline stages on the |> operator; do not conflate them with actor pipe. |
| [spec #52](https://github.com/ores-truffle-oreslang/attolang-language-spec.md/pull/52), Nov 14 | Recognize boolean ^^ continuations and avoid mistaking XOR for a single-caret return. |
| [Java #3009](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/3009), Nov 14 | Pin the actual XOR truth-table source: Git blob f7e1e082572a0d63483acca44ee7b61a9e6e9032. |
| [LLVM #1728](https://github.com/ores-truffle-oreslang/oreslang-source.llvm/pull/1728), Nov 16 | Preserve &&, ^^, || operator precedence spelling, not word aliases. |
| [.NET #521](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/521), Nov 14 | Structured frontend also admits &&, ^^, ||; scalar int frontend still has no boolean XOR guest parity. |
| [spec #68](https://github.com/ores-truffle-oreslang/attolang-language-spec.md/pull/68), Nov 15 | Reject opening a 129th nested block-comment level, with accurate source line. |
| [LLVM #1834](https://github.com/ores-truffle-oreslang/oreslang-source.llvm/pull/1834), [#1839](https://github.com/ores-truffle-oreslang/oreslang-source.llvm/pull/1839), and [.NET #540](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/540) | Reject raw ESC/DEL and bidi spoofing controls before formatting source, even in comments or literals. |
| [LLVM #1854](https://github.com/ores-truffle-oreslang/oreslang-source.llvm/pull/1854), Nov 16 | Respect contextual identifiers (borrow, critical, gen, pure, requires, serial, structural, suspend, using). |
| [.NET #503](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/503), [#523](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/523) and [#525](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/525) | Preserve supported end-body headers, canonicalize required name-first parameters using established formatter policy, but never erase an optional x?: Type marker. |

## Coverage added

- tests/multiline_actor_io_return.rs — split actor pipelines, directional I/O, caret returns and state isolation.
- tests/recent_merge_compatibility_20261116.rs — XOR multiline formatting, comment depth boundary conditions, source-control preflight, contextual operator names, 2–16 actor stages, inline functional lambda stages, mixed parameters and actual merged Java fixture.
- tests/cli_alias_parity.rs — both binaries reject malicious inputs without emitting partial stdout.
- tests/fixtures/merged-java-logical-punctuation-v1.atto — exact bytes of the merged Java #3009 file, verified by matching SHA.

## Versioned contract mismatch

**Single caret is not accepted by the currently merged Java/LLVM grammar.** Java #3009 and LLVM #1728 reserve single ^ while supporting ^^ for boolean XOR. The proposed ^ return shorthand is preserved by the formatter branch for design and future parser work; it is not compiler-verified or executable guest syntax.

Likewise, << and >> single-endpoint I/O and actor-to-stream pipe are only formatter-preservation contracts here. Runtime behavior and all-backend typechecking need separate source-language conformance and guest execution.

## Evidence and merge gate

A successful formatter test only proves lexical formatting and fixed-point output, not Java/LLVM/.NET acceptance or native-AOT execution. Require real, nonempty exact-head Rust/CLI test and Clippy jobs before promoting the PR. Do not copy private Git trees to another GitHub organization without an approved security transfer.
