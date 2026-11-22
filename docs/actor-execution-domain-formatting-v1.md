# Actor execution class formatting v1 (target)

Authority: [atto-spec actor execution classes](https://github.com/atto-lang/atto-spec/pull/49) and the accompanying [conformance draft](https://github.com/atto-lang/atto-platform-conformance/pull/43).

- Preserve all three domain spellings `shared`, `isolated`, `untrusted`. Omitted domain means `shared`; it is NOT evidence of trusted host admission.
- Preserve `pub`, `persistent`/`durable`, `serial`, and `http`/`tcp`/`ws` in declaration headers, without reordering uncertain grammar.
- Do not rewrite a conflicting or repeated execution class into a valid declaration. Invalid source is for the parser/checker to reject.
- The two existing bounded lexical normalizers are not full actor-header parsers. Tests only assert token preservation, comment/string opacity and idempotence.
- Future normalization of explicit `define shared actor` to `define actor` is permitted **only** when a real compiler verifies parse-equivalence AND equal guest artifacts including actor-domain/security metadata. Never silently change `isolated` or `untrusted`, or use a stub/host-only reference test as compiler proof.
- Until that gate is implemented, keep explicit `shared` verbatim. Mark guest compiler and runtime execution status `NOT_EXECUTED`.

No verified parser, actor runtime, hostile sandbox or guest byte-equivalence is implied by these lexical tests.
