# `unsafe` block formatting target v1

**TARGET ONLY — format policy, not proof of executable unsafe compiler support (2026-11-19).**
Normative authority: [atto-spec: unsafe-blocks-v1](https://github.com/atto-lang/atto-spec/blob/feat/unsafe-block-contract-v1-20261119/language/unsafe-blocks-v1.md).

## Canonical layout

```atto
pub fnc main(): int {
    unsafe {
        val int answer = 20 + 22;
        stdio.println(answer);
    }
    return 42;
}
```

- Preserve `unsafe` as a standalone whole token immediately followed by one space and an opening brace (`unsafe {`); use the same brace/newline conventions as other Atto statement blocks.
- Indent nested statements consistently with ordinary Atto blocks; keep matching closing brace at the block's original indentation. Nested `unsafe` blocks do not collapse.
- Retain `unsafe fnc` and `pub unsafe fnc` tokens in the callable header; do **not** insert or remove an unsafe callable requirement from a signature. Keep independent `susp`/other modifiers and their order per the language spec.
- Do not convert a block statement to an expression, drop semicolons, move a statement outside the block, move a call into it, elide an empty block, or extend the scope of `unsafe` across a declaration/lambda/function boundary.
- Do not normalize identifiers containing `unsafe` (e.g. `unsafe_tools`), comments, strings or `#unsafe` Symbols. Quoted text may contain `unsafe { }` literally.
- Preserve comments directly adjacent to opening or closing braces and preserve source maps. Formatters that cannot parse nested braces/comments/strings correctly must leave uncertain source unchanged or report a typed unsupported formatting result; do not attempt a regex rewrite.
- Whitespace changes require parser round-trip/AST equivalence in a supported compiler. Verified format admission additionally requires the *real* compiled guest code to be behaviorally equivalent on the exact source/target. Atto browser/Native G0 may reject the token entirely; that does not mean the formatter can silently remove it.

## Negative/near-miss regression corpus

```atto
// unsafe { } — comment, not a block
pub fnc main(): int {
    val string literal = "unsafe { is ordinary text }";
    val int unsafe_tools = 42;
    unsafe {
        // Two inner scopes must remain distinct.
        unsafe {
            stdio.println(literal);
        }
    }
    return unsafe_tools;
}
```

Format validation should cover duplicate/nested blocks, blank blocks, escaped quotes, nested block comments, Unicode identifiers, strings, Symbols, CRLF, large bounded source inputs and idempotence. This document introduces **no implementation** of unsafe grammar; only a target formatter contract. No browser compiler, Java/Graal, LLVM, Forge, .NET, independent Wasm, Cranelift or JS guest test is certified by this documentation.
