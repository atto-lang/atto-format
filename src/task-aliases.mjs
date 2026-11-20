/** Inline task formatting contract (atto-spec typed-task-payload-sync-v1).
 *
 * 'do task<T>' spawns an independently supervised child, whereas
 * 'do sync task<T>' executes inline in the caller's logical task.
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
