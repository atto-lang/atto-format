# Legacy Oreslang formatter -> canonical Atto specification alignment v1

**Status: consumer tracking, not a completed cross-formatter, semantic-AST or compiler-guest parity proof.** The canonical normative grammar and language rules are at atto-lang/atto-spec. This repository is a distinct historical formatter implementation; the Atto-named counterpart is atto-lang/atto-format.

This file and contracts/v1/atto-spec-consumer-v1.json pin atto-spec PR #115 commit 2f32c957ce01bc157479a5c96538efd853dc3643 and rule-registry Git blob 987dfab669a0cc62b9be2eac5c968116df058a3a. These are explicit snapshot dependencies, not a live auto-sync mechanism. Reconcile any changed rule IDs before asserting a later edition.

## Minimum guarantees

- Use AttoLang .atto for new fixture source; .ores remains expressly migration compatibility. An extension does not itself prove parsing/conformance.
- Preserve copy! versus copy!!, take!, drop!, loan qualifiers, effects, const initializer class, and explicit callable returns. Context-dependent lifetime! / lt! or method canonicalization is allowed only with a valid parsed context.
- No reordering/deleting a late or duplicate use namespace declaration to convert invalid code into valid code. Derived namespace resolution must receive the original logical path, not machine-dependent symlink/host path.
- For accepted source, require F(F(s))=F(s) and an independently validated equivalent semantic AST before and after formatting for the same edition/profile/source path. Invalid source must either be rejected or remain invalid for an equivalent condition.
- Because the legacy codebase has no established complete cross-backend semantic AST adapter/golden receipt, its existing docs/SEMANTIC_AST_PARITY_GATE.md continues to mark this condition unexecuted; no lexical approximation or source-free mock counts.
- Full parity requires separately verified original and formatted compiler/guest runs per backend, exact source/formatter/compiler trees and actual observed output. Do not transfer results from atto-format simply because the repository is its legacy counterpart.

The additional Rust test only checks local ownership operator spelling and idempotence on a narrow source. It is NOT a proof of namespace validity, type preservation, semantic AST equivalence, compiled guest or cross-repo parity. Report these separately as NOT_EXECUTED.

Migration should combine nonoverlapping legacy fixes after diff/semantic audit; avoid deleting unique tests or claiming mirrored Git history without SHA verification.
