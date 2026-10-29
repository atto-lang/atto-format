# Change summary

Describe the behavior changed, affected Oreslang language/runtime surfaces, and why the change is necessary. Reference related upstream/compiler PRs when relevant.

## Compatibility and safety

- [ ] Native `.ores` semantics are the source of truth for backend-neutral code, or the host-only boundary is explained.
- [ ] Relevant Java/Graal, LLVM, .NET, Wasm, Cranelift, Forge, or other backend implications are stated; unsupported features fail closed instead of silently degrading.
- [ ] Where relevant, actor ownership/isolation, non-reentrancy, async cancellation, borrow lifetimes, and bounded resource/real-time constraints are tested.
- [ ] No implicit conversions, operator-overload assumptions, or nested Future/Option auto-flattening are introduced.
- [ ] Not applicable items above are explained rather than claimed to pass.

## Verification evidence

- **Exact PR head SHA / Git tree:** 
- **Executed test commands / workflow run and job IDs:** 
- **Actual nonzero runner steps and result counts (pass/fail/skip):** 
- **Compiled Oreslang guest behavior vs host-only tests:** 
- **Cross-backend parity differences and follow-up issues:** 

A queued, skipped, or zero-step workflow does **not** count as executed CI. A successful host-only test does **not** establish compiled guest semantics. Report limitations explicitly.

## Review and source handling

- [ ] CODEOWNERS, required reviews, protected-branch rules, and publication gates remain effective; this PR does not bypass them.
- [ ] Any transfer of private source to an external CI carrier has separately approved source and destination authority, exact-tree verification, and no cross-organization checkout credentials. Otherwise, no private-source export occurs.
- [ ] Merge dependencies and post-merge verification are documented where needed.
