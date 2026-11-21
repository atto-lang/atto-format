//! Canonicalize legacy exceptional statements without rewriting comments,
//! strings, effect clauses, or similarly named identifiers.
//!
//! The spelling pass is intentionally lexical: whether the backend can lower
//! the exceptional operation is a separate, checked compiler decision.

use crate::{LexState, scan_visible, visible_word_spans};

pub(super) fn canonicalize(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut lex = LexState::default();

    for (line_number, line) in source.split('\n').enumerate() {
        if line_number != 0 {
            output.push('\n');
        }

        let words = visible_word_spans(line, lex);
        let mut next_lex = lex;
        let _ = scan_visible(line, &mut next_lex);
        lex = next_lex;

        let mut rewritten = line.to_owned();
        // Work from right to left so the original byte offsets stay valid.
        for (index, span) in words.iter().enumerate().rev() {
            let word = &line[span.start..span.end];
            if !matches!(word, "throw" | "raise" | "panic") {
                continue;
            }

            let prefix = line[..span.start].trim_end();
            let boundary = prefix.is_empty()
                || prefix.ends_with(';')
                || prefix.ends_with('{')
                || prefix.ends_with('}')
                || prefix.ends_with("::")
                || prefix.ends_with("->")
                || prefix.ends_with("=>")
                || words.get(index.wrapping_sub(1)).is_some_and(|previous| {
                    let prior_word = &line[previous.start..previous.end];
                    matches!(prior_word, "then" | "else" | "do" | "begin")
                        && line[previous.end..span.start].trim().is_empty()
                });
            if !boundary
                || prefix.ends_with('.')
                || prefix.ends_with('#')
                || prefix.ends_with('@')
            {
                continue;
            }

            // A contiguous ! is already canonical. An adjacent logical-not
            // expression (with whitespace) must not be silently reinterpreted
            // as the exceptional bang token.
            let following = &line[span.end..];
            if following.trim_start().starts_with('!') {
                continue;
            }

            rewritten.insert(span.end, '!');
        }

        output.push_str(&rewritten);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::canonicalize;

    #[test]
    fn masks_literals_comments_and_members() {
        let source = r#"throw new Error();
// raise new Error();
/* panic new Error();
   throw new Error(); */
const text = "panic new Error();";
object.raise(new Error());
panic new Error();
"#;
        let result = canonicalize(source);
        assert!(result.starts_with("throw! new Error();"));
        assert!(result.contains("// raise new Error();"));
        assert!(result.contains("/* panic new Error();"));
        assert!(result.contains("   throw new Error(); */"));
        assert!(result.contains("\"panic new Error();\""));
        assert!(result.contains("object.raise(new Error());"));
        assert!(result.contains("panic! new Error();"));
        assert_eq!(canonicalize(&result), result);
    }

    #[test]
    fn handles_inline_statement_boundaries_without_changing_logical_not() {
        let source = "if ready then throw new Error(); fi\nbegin raise new Error();\nthrow !error;\n";
        let result = canonicalize(source);
        assert!(result.contains("then throw! new Error();"));
        assert!(result.contains("begin raise! new Error();"));
        assert!(result.contains("throw !error;"));
    }
}
