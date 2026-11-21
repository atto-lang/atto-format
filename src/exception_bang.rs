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

        // Assemble spans once: repeated insertions would be quadratic for a
        // large one-line sequence of exceptional statements.
        let mut rewritten = String::with_capacity(line.len());
        let mut cursor = 0usize;
        for (index, span) in words.iter().enumerate() {
            let word = &line[span.start..span.end];
            if !matches!(word, "throw" | "raise" | "panic") {
                continue;
            }

            let prefix = line[..span.start].trim_end();
            let boundary = prefix.is_empty()
                || prefix.ends_with(';')
                || prefix.ends_with('{')
                || prefix.ends_with('}')
                || words.get(index.wrapping_sub(1)).is_some_and(|previous| {
                    let prior_word = &line[previous.start..previous.end];
                    matches!(prior_word, "then" | "else" | "do" | "begin")
                        && !line[..previous.start].trim_end().ends_with('.')
                        && line[previous.end..span.start].trim().is_empty()
                });
            if !boundary
                || prefix.ends_with('.')
                || prefix.ends_with('#')
                || prefix.ends_with('@')
            {
                continue;
            }

            // Require an unambiguous operand on THIS physical line. A
            // bare keyword followed by a newline and `!expression` is the
            // migration spelling plus logical-not, never the bang token.
            // Treat comment-separated operands, call-like identifiers, and
            // punctuation-only tails as ambiguous until parser verification.
            let following = line[span.end..].trim_start();
            if following.is_empty()
                || following.starts_with('!')
                || following.starts_with("/*")
                || following.starts_with("//")
                || following
                    .chars()
                    .next()
                    .is_some_and(|c| matches!(c, ';' | ':' | '=' | '.' | '(' | ')' | ',' | '}'))
            {
                continue;
            }

            rewritten.push_str(&line[cursor..span.end]);
            rewritten.push('!');
            cursor = span.end;
        }

        rewritten.push_str(&line[cursor..]);
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

    #[test]
    fn never_conflates_multiline_logical_not_with_an_exception_bang() {
        let source = "throw\n  !ready;\nraise // comment\n  Error(1);\npanic;\n";
        assert_eq!(canonicalize(source), source);
    }

    #[test]
    fn leave_ambiguous_call_syntax_and_commented_operands_unchanged() {
        let source = "throw(new Error());\nraise /* why */ error;\npanic::value;\nfoo::throw new Error();\nvalue -> raise new Error();\ncase => panic new Error();\n";
        assert_eq!(canonicalize(source), source);
    }

    #[test]
    fn preserves_unicode_offsets_and_stays_linear_for_long_lines() {
        let prefix = "\"🤖\"; throw new Error();";
        assert_eq!(canonicalize(prefix), "\"🤖\"; throw! new Error();");
        let source = "throw e; ".repeat(4096);
        let result = canonicalize(&source);
        assert_eq!(result.matches("throw! e;").count(), 4096);
        assert_eq!(canonicalize(&result), result);
    }

}
