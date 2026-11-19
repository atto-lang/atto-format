# Atto specification rule consumer v1

**Status: proposed consumer alignment; the new tests exercise only a local, conservative lexical ownership normalizer. This is NOT proof of full parser, AST or compiled Atto guest conformance.**

Canonical source: atto-lang/atto-spec, language/specification-framework-v1.md and contracts/specification-rules-v1.json at commit dab563bedaaf2ce801b2a2991e3c693f664bb2ab (registry Git blob 987dfab669a0cc62b9be2eac5c968116df058a3a). A pinned spec snapshot is an input/version reference; it does not stay synced automatically. On every spec revision, review changed IDs and rebase the consumer contract before advertising compatibility.

This repository and historical ores-truffle-oreslang/oreslang-format are distinct implementations with distinct capability coverage. Only atto-spec may change normative syntax/effects/ownership; a formatter cannot authorize an unimplemented grammar extension. Existing callable-header/ownership/return-formatting tests continue to apply.

## Mandatory gates before full rule conformance

1. Feed the same edition, feature profile, package-relative logical path and source to a real parser/typechecker both before and after formatting.
2. If original source is valid, format successfully, format to a fixed point, and obtain a canonical semantic AST equivalent in types, bindings, effects, ownership, startup-initializer class, actor state and operator identity. Compare actual executed guests for runtime behavior separately.
3. If source is invalid, format must reject or preserve the equivalent invalid condition (including duplicate/late namespace, missing return type, forbidden ownership transition). Never silently correct a compile error into an accepted program.
4. Only rewrite lifetime! -> lt! when the binder is syntactically admitted and its meaning is checked. Preserve copy! versus copy!!, take! and drop!, comments, literals and static env-based constants.
5. Pin original/formatted fixture blobs, formatter and compiler exact commit/tree, edition and profile, semantic adapter SHA, actual run outputs, and backend target. Mark full semantic/parity checks NOT_EXECUTED until authenticated real compiler adapters are available; Node-only checks do not establish them.

The companion contracts/spec-rule-consumer-v1.json records provenance and outstanding evidence. The test suite checks contract integrity and safe operations of the existing lexical normalizer only; it does not claim to implement a full Atto parser. See atto-platform-conformance for portable guest results.
