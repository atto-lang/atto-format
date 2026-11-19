<!-- org-merge-review-policy:begin -->
> **Mandatory agent policy:** Read [ores-truffle-oreslang/.github/AGENTS.md](https://github.com/ores-truffle-oreslang/.github/blob/main/AGENTS.md) before editing, reviewing, or merging. **0–72h:** Codex/ChatGPT + Claude/Opus + `ORESoftware` + `the1mills` reviews. **72h–<240h:** one verified independent human/AI approval permits merging with passing tests and no conflicts. **240h+:** fresh post-240h reviews from all four again. Every phase requires green verified CI, current-head evidence, no holds/critical risks, and compliance with GitHub rules. Merge eligible PRs immediately; never reset age or bypass protections. More restrictive local safety rules remain effective.
<!-- org-merge-review-policy:end -->

# AttoLang source-first implementation guidance

These instructions apply to this repository's code changes, examples, fixtures, and AI coding agents. They supplement, and never override, repository security rules or review requirements.

## Source placement

- New AttoLang guest source, standard-library algorithms, sample programs, and guest-language regression fixtures use **`.atto`**. The `.ores` suffix is for legacy compatibility or explicit migration tests only; do not create new `.ores` features.
- Implement portable guest-visible behavior in AttoLang wherever it is expressible. Add only the smallest necessary host/compiler primitive when the language cannot safely implement that behavior itself. Do not disguise host-language emulation as actual AttoLang guest execution.
- Keep functions and methods in `snake_case`. Preserve actor ownership, sealed state, capability boundaries, deterministic errors, and full-AOT compatibility.
- If a necessary host-language implementation remains, document why guest code is not yet possible, which capability it supplies, and how it can be replaced. Do not mass-rename host compiler/runtime files to `.atto`.

## Evidence and security

- Separate source-only fixtures, host-model tests, compiler parse/typecheck, executable guest tests, and native AOT evidence. A zero-step workflow, synthetic receipt, or host-only test means **NOT_EXECUTED** for guest parity.
- Record exact fixture/compiler Git blobs and source heads when making parity claims; do not mark another backend as passing without its own executed result.
- Do not copy private Git objects to another organization simply because its runner has capacity. Respect source-side authorization, independent security review, approved private destination, and protected-branch requirements.

## Formatter-specific responsibilities

- The formatter may remain implemented in its established host tooling language; this directive does not require rewriting a parser or formatter in `.atto`.
- Treat `.atto` as the canonical language-source extension in discovery, examples and round-trip fixtures; keep `.ores` only for intentional legacy compatibility. Reject unrelated file extensions rather than silently formatting them.
- Preserve canonical AttoLang syntax and semantics: `snake_case` names, `define pub`, explicit `initialize(...) -> void`, presence bindings `if ... as const/let`, and safe `final`/`const` normalization. Never claim a formatting pass establishes compiler acceptance.
- Record idempotence and AST-equivalence separately from actual backend guest execution. Unsupported compiler `--analyze` must remain inconclusive rather than being marked clean.
