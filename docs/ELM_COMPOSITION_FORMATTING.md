# Elm-inspired composition formatting contract

This draft adds round-trip tests for `map`, `flat_map`, `and_then`, `map_err`, `filter_map`, `on_success` and nested callback type parameters. Oreslang public fnc/method names are **snake_case**.

The formatter **does not** mechanically rewrite historic `flatMap`/`onSuccess`: those identifiers are retained for compatibility until compiler and formatter migrations have explicit equivalence tests. Type checking, ownership validation and effect enforcement remain compiler responsibilities, not formatter guesses.

The fixtures protect lexical preservation and idempotence only, not runtime validity of every showcased library operation. Cross-repo semantics and fail-closed backend admission: common-source PR #36, Java #1017, LLVM #413, RX #12.
