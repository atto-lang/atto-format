# Literal-only `final const` alias: fail-closed integration contract

For direct class fields, the **bounded** compatibility syntax `final const int x: 5` is
semantically the same fixed-slot declaration as `final int x: 5`. The formatter
may remove `const` only for unambiguously lexical literals in supported
primitive types (`int`, `bool`, `string`). The rewrite is scoped to direct
class members and preserves comments/strings. It does **not** rewrite actors,
locals, module bindings, method bodies, arithmetic, negatives, references,
calls, mutable referents, or more complex field types.

The Java/GraalVM parser should admit literal-only `final const` class fields
as the same fixed slot as `final`, but reject nonliteral initializers with
an explicit diagnostic. LLVM admits only its existing i64-literal field subset.
Neither compiler currently has an audited general-purpose compile-time
constant evaluator for class fields. Arithmetic, named constants, symbol
literals, type aliases, and cross-module values need a shared typed
constant-evaluation contract before expanding this rewrite.

**Important migration warning:** The existing Java compiler historically uses
`const` as a fixed/read-only binding (including runtime initialized locals).
Do **not** silently reinterpret every `const` binding as a compile-time
constant: that is a separate potentially breaking language change.

**Inheritance blocker:** The fixed-slot bit and prohibition against subclass
redeclaration are different obligations. Java PR #890 retains an explicit
`finalDeclaration` AST bit and adds a transitive final-shadowing diagnostic.
The current storage layout already disallows **all** same-name inherited
fields, even ordinary `const`, regardless of compatible type. Therefore the
separately proposed "subclass can explicitly shadow const" semantics are
**not yet implemented**; accepting that requires a distinct static-lookup
versus storage-slot design, backend layout parity, and ABI tests. The LLVM
subset does not yet support class inheritance.

**Release gate:** Keep the formatter/backend PRs draft until exact-head Rust
fmt/test/clippy, Java full suite, LLVM CTest/Clang smoke, and cross-backend
parse/format/reparse parity have passed. Do not claim green from a non-executing
GitHub Actions run.
