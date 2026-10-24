//! Canonicalize multiple executable statements on one physical line.
//!
//! Only split top-level semicolons. Semicolons in strings, comments, calls,
//! indexing, or inline record/struct literals are not statement boundaries.
//! Formatting is not syntax validation: the compiler owns grammar diagnostics.

use crate::LexState;

pub(crate) fn normalize(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut lex = LexState::default();

    for (line_no, line) in source.split('\n').enumerate() {
        if line_no != 0 {
            out.push('\n');
        }
        let mut chars = line.char_indices().peekable();
        let (mut parens, mut brackets, mut braces) = (0usize, 0usize, 0usize);
        let mut cuts = Vec::new();

        while let Some((at, ch)) = chars.next() {
            let next = chars.peek().map(|&(_, c)| c);
            if lex.block_comment {
                if ch == '*' && next == Some('/') {
                    chars.next();
                    lex.block_comment = false;
                }
                continue;
            }
            if let Some(quote) = lex.quote {
                if lex.escape {
                    lex.escape = false;
                } else if ch == '\\' {
                    lex.escape = true;
                } else if ch == quote {
                    lex.quote = None;
                }
                continue;
            }
            if ch == '/' && next == Some('/') {
                break;
            }
            if ch == '/' && next == Some('*') {
                chars.next();
                lex.block_comment = true;
                continue;
            }
            if matches!(ch, '"' | '\'' | '`') {
                lex.quote = Some(ch);
                continue;
            }

            match ch {
                '(' => parens += 1,
                ')' => parens = parens.saturating_sub(1),
                '[' => brackets += 1,
                ']' => brackets = brackets.saturating_sub(1),
                '{' => braces += 1,
                '}' => braces = braces.saturating_sub(1),
                ';' if parens == 0 && brackets == 0 && braces == 0 => {
                    let remainder = line[at + 1..].trim_start();
                    // 'if condition; then' is a supported compatibility
                    // spelling. Never terminate its header. Keep comments
                    // attached to the original statement.
                    if !remainder.is_empty()
                        && !remainder.starts_with("//")
                        && !remainder.starts_with("/*")
                        && !crate::starts_word(remainder, "then")
                        && !crate::starts_word(remainder, "do")
                    {
                        cuts.push(at + 1);
                    }
                }
                _ => {}
            }
        }

        let mut last = 0usize;
        for cut in cuts {
            out.push_str(line[last..cut].trim_end());
            out.push('\n');
            last = cut;
        }
        out.push_str(&line[last..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn separates_top_level_actions() {
        assert_eq!(
            normalize("first(); second(); third();"),
            "first();\n second();\n third();"
        );
    }

    #[test]
    fn preserves_opaque_semicolons_and_nested_expressions() {
        let source = "call(\"one;two\", [1; 2]); next(); // last; word";
        assert_eq!(
            normalize(source),
            "call(\"one;two\", [1; 2]);\n next(); // last; word"
        );
        let record = "const v = infer struct {left: \"a;b\"}; run();";
        assert_eq!(
            normalize(record),
            "const v = infer struct {left: \"a;b\"};\n run();"
        );
    }

    #[test]
    fn header_semicolon_is_not_a_statement_boundary() {
        assert_eq!(normalize("if ready; then run();"), "if ready; then run();");
        assert_eq!(
            normalize("for (i = 0; i < 3; i++) do work();"),
            "for (i = 0; i < 3; i++) do work();"
        );
    }

    #[test]
    fn comments_never_create_actions() {
        let source = "call(); // fake(); next();\n/* comment; */ more();";
        assert_eq!(normalize(source), source);
    }
}
