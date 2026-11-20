/** Inline task formatting contract (atto-spec typed-task-payload-sync-v1).
 *
 * 'do task<T>' spawns an independently supervised child, whereas
 * 'do sync task<void>' (or inferred void) runs inline in the caller's task.
 * Nonvoid 'do sync task<T>' is an error for the compiler, not a formatter
 * rewrite opportunity. Bare 'sync task {};' is invalid statement syntax;
 * the formatter preserves it unchanged for proper parser diagnostics.
 * Erasing 'sync' changes execution order, cancellation ownership,
 * borrowing, supervision and the permitted suspension effects.
 *
 * The old normalize_task_aliases() entry point remains for source/API
 * compatibility, but there is NO valid task alias to rewrite under this
 * contract. This function must preserve source text exactly, including
 * comments, nested generics, literals and malformed expressions.
 * Other formatting passes may normalize whitespace only when they
 * prove equivalent grammar/effects; they cannot reintroduce aliasing.
 *
 * Full compiler parse/typed-MIR and guest conformance are still pending.
 */
export function normalize_task_aliases(source) {
  return source;
}
