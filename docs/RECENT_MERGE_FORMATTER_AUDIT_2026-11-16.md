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

## ADT, structural-pattern, callable and suspension audit extension

Additional upstream sources merged between November 11 and November 16:

| PR | Formatter responsibility |
| --- | --- |
| [Java #2824](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/2824) | Preserve nested Option/Result/Option sum tags and numeric comparison syntax. |
| [Java #2715](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/2715), [#2727](https://github.com/ores-truffle-oreslang/oreslang-source.java/pull/2727) | Preserve old tilde records and default-match forms without unsafe collection-constructor migration. |
| [.NET enum #469](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/469), [#472](https://github.com/ores-truffle-oreslang/oreslang-source.net/pull/472) | Keep nominal enum owner/variant spelling and exhaustive match arms. Limited opt-in compiler, not general ADT parity. |
| [Rust/Wasm #56](https://github.com/atto-lang/atto-source.rs/pull/56), [#63](https://github.com/atto-lang/atto-source.rs/pull/63) | Preserve exhaustive Result/Option and nested Option<Result> tag patterns and non-exhaustive negative fixtures. |
| [Rust/Wasm #191](https://github.com/atto-lang/atto-source.rs/pull/191) | Retain exact-value match arms, binding guards, return-match and fallback spellings. |
| [Atto spec #16](https://github.com/atto-lang/atto-spec/pull/16) | Preserve List/Tuple rest wildcards and explicit bindings, but do not claim compiler implementation. |
| [Atto spec #19](https://github.com/atto-lang/atto-spec/pull/19) | Keep out-of-line mtd receiver syntax separate from method formals, and distinguish brace bodies from comma/end bodies. |
| [Atto spec #37](https://github.com/atto-lang/atto-spec/pull/37) | Recognize susp, suspends, suspend without rewriting legacy code into unsupported backend spelling. |
| [LLVM #1647](https://github.com/ores-truffle-oreslang/oreslang-source.llvm/pull/1647) | Structural patterns have compiler-negative gating, not actual LLVM guest pattern execution. |

### Exact-source ADT fixture provenance

These 15 fixture files were copied from merged upstream compiler trees into
tests/fixtures/adt-merge-window. The Git blob SHAs were checked against the
source (and are visible in GitHub). They are tested for formatting only.
The original compiled-guest or compiler-negative classifications are preserved.

| Fixture basename | Original Git blob SHA |
| --- | --- |
| java-numeric-sum-equality | c9e68a0c4df5f25ebc217361fd67557558a3b2bc |
| net-nullary-enum | 619408dc388499cbcb53e1509e05aacea8074044 |
| net-enum-function | e2b6c2d0b6060a55e328c3310deea44d607f0e9e |
| rs-result-ok | 6ffc3d2d799938b9e7d4f1856e0c191c9ac71ee6 |
| rs-result-err | d4455d4edac14eb17094d9a87076a2edc5ad8061 |
| rs-option-some | 84141b497c58ce6e4df4f75da368004899828d8d |
| rs-option-none | 40d65be39d67de284cf68054a6a31f6038d86498 |
| rs-nested-option-result-ok | ecde4abd6420862d7c0c1975350241b78e88181d |
| rs-nested-option-result-err | c5a12acda9c71a4a600ef8c18635967fb18ba272 |
| rs-nested-option-result-none | 5e078f8601d9de859111a6cbbec6a9d6279ab1ed |
| rs-nested-option-result-arguments | 9e3440f2e1883ee304b8b69f48e27ffb59ef0d0f |
| rs-fail-option-wrong-variant | f3d56507736efd512a4cfbadde08f8d9407ff8f8 |
| rs-fail-result-duplicate-arm | 7409c2b63cbd8d41a493ce89291305155e1231e5 |
| rs-fail-nested-nonexhaustive | 5252768fe8ff766169a0e4ec7e35172b1cc4a24a |
| rs-fail-adt-identity-equality | 0adbb57cfc4e4fe15ff52dd8af7996223aa18833 |

### Fixes and deliberate non-claims

- A structural ADT pattern brace followed by a binder or guard is not the
  executable match-arm brace. The normalizer now scans for the next arm arrow.
- mtd declarations have two parameter groups. Only the method arguments are
  subject to ordinary parameter reordering; explicit mutable self stays mutable.
  Equivalent named receivers normalize consistently, and compact comma/end
  bodies retain their implicit tail instead of acquiring a return or semicolon.
- Colon-typed brace bodies with the second arrow must not be rewritten into an
  invalid double-arrow return type.
- Historical tilde aggregate constructor syntax is not automatically rewritten
  into modern Map.of, Tuple.of and Array.of builders without ownership proof.
- Actor-supervisor ADT messages, typed structural patterns, sequence-rest
  matching, single-caret return, and many mtd receiver spellings still require
  actual backend parser, typecheck and guest-execution evidence. Formatting
  is not an implementation of their runtime semantics.

New regression modules: tests/adt_merge_window.rs,
tests/modern_callable_receivers.rs and src/match_switch.rs local unit tests;
the two CLI binaries also replay representative upstream ADT sources.

## Follow-on directional pipeline proposal: rpipe and reverse value application

[atto-spec PR #45](https://github.com/atto-lang/atto-spec/pull/45) remains an
**unmerged draft candidate**. Its directional invariant is lexical:
`A pipe B` connects A → B; `A rpipe B` connects B → A. Written
reverse-stage order does **not** invert the actual transform or runtime
dataflow. `value |> f` and `f <| value` are single-value functional
application, never a continuous graph or stream pump.

The formatter now recognizes `rpipe` alongside `pipe` in both leading
and trailing line-continuation positions; recognizes both `|>` and `<|`
symbolic continuations; and indents split actor bindings consistently.
The code preserves contextual identifiers named `rpipe` and `pipe`,
literal/comment text, and nested generic type closers without rewriting
operator spelling. The proposed inline `(let result: Stream<T> =
Stream.buffered<T>(capacity))` graph endpoint remains a parenthesized
capture and is not converted into a single-value `<<`/`>>` operation.

Source tests: `tests/directional_pipe_rpipe_application.rs` (13 cases),
`tests/fixtures/directional-pipelines-v1.atto`, and
`tests/cli_alias_parity.rs` (both binaries, --check and stdout parity).
These are formatting/idempotence checks only. The PR does not infer edge
roles, reject mixed pipe/rpipe source at typecheck time, create PipeHandles
or execute live graph/stream/actor guests. A genuine acceptance claim
requires compiled and run source tests on the backend's exact head.
