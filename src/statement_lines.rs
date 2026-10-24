//! Canonicalize multiple executable statements on one physical line.
//!
//! Only split top-level semicolons. Semicolons in strings, comments, calls,
//! indexing, or inline record/struct literals are not statement boundaries.
//! Formatting is not syntax validation: the compiler owns grammar diagnostics.

use crate::LexState;

// Skip only whitespace and leading comments after a statement terminator.
// Unlike rescanning the entire suffix at every semicolon, this has linear
// total cost across ordinary and generated long lines. Quoted literals are
// executable tokens here, never mistaken for comments.
fn next_code_after_comments(mut remaining: &str) -> Option<&str> {
    loop {
        remaining = remaining.trim_start();
        if remaining.is_empty() || remaining.starts_with("//") {
            return None;
        }
        if let Some(comment) = remaining.strip_prefix("/*") {
            let end = comment.find("*/")?;
            remaining = &comment[end + 2..];
            continue;
        }
        return Some(remaining);
    }
}

// Track opaque data braces across physical lines. Ordinary executable
// control/callable braces do not inhibit statement splitting in their bodies;
// inline data/record braces must, even when their fields span multiple lines.
fn starts_data_brace(prefix: &str) -> bool {
    let before = prefix.trim_end();
    if before.ends_with('=') || before.ends_with(':') {
        return true;
    }
    let tail = before
        .rsplit(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .next()
        .unwrap_or("");
    matches!(tail, "struct" | "record" | "state")
}

pub(crate) fn normalize(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut lex = LexState::default();
    let (mut parens, mut brackets, mut opaque_data_braces) = (0usize, 0usize, 0usize);

    for (line_no, line) in source.split('\n').enumerate() {
        if line_no != 0 {
            out.push('\n');
        }
        let mut chars = line.char_indices().peekable();
        // Local executable braces only suppress splitting while still on the
        // same source line. Across a newline, their bodies are statements.
        let mut inline_executable_braces = 0usize;
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
                '{' if opaque_data_braces > 0 || starts_data_brace(&line[..at]) => {
                    opaque_data_braces += 1;
                }
                '{' => inline_executable_braces += 1,
                '}' if opaque_data_braces > 0 => opaque_data_braces -= 1,
                '}' => inline_executable_braces = inline_executable_braces.saturating_sub(1),
                ';' if parens == 0
                    && brackets == 0
                    && opaque_data_braces == 0
                    && inline_executable_braces == 0 => {
                    let remainder = line[at + 1..].trim_start();
                    // 'if condition; then' is a supported compatibility
                    // spelling. Never terminate its header. Keep comments
                    // attached to the original statement.
                    // A trailing /* block comment */ alone belongs to
                    // this statement, but a comment followed by executable
                    // source still begins another statement. Inspect the
                    // visible tail instead of suppressing every /* prefix.
                    if let Some(next_code) = next_code_after_comments(remainder) {
                        if !crate::starts_word(next_code, "then")
                            && !crate::starts_word(next_code, "do")
                        {
                            cuts.push(at + 1);
                        }
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
    fn block_comment_between_actions_does_not_hide_the_second_action() {
        assert_eq!(
            normalize("first(); /* keep note */ second();"),
            "first();\n /* keep note */ second();"
        );
        assert_eq!(
            normalize("first(); /* keep note */"),
            "first(); /* keep note */"
        );
        assert_eq!(
            normalize("first(); /* keep note */ // trailing"),
            "first(); /* keep note */ // trailing"
        );
        assert_eq!(
            normalize("first(); /* start\n continued */ second();"),
            "first(); /* start\n continued */ second();"
        );
    }

    #[test]
    fn block_comments_before_header_terminators_stay_on_one_line() {
        assert_eq!(
            normalize("if ready; /* header note */ then run();"),
            "if ready; /* header note */ then run();"
        );
        assert_eq!(
            normalize("first(); /* one */ /* two */ second();"),
            "first();\n /* one */ /* two */ second();"
        );
        assert_eq!(
            normalize("first(); /* one */ // second();"),
            "first(); /* one */ // second();"
        );
        assert_eq!(
            normalize("first(); /* unterminated"),
            "first(); /* unterminated"
        );
    }

    #[test]
    fn many_actions_do_not_require_rescanning_long_suffixes() {
        let line = "action(); ".repeat(2048);
        let separated = normalize(&line);
        assert_eq!(separated.lines().count(), 2048);
        assert!(separated.lines().all(|line| line.trim() == "action();"));
    }

    #[test]
    fn comments_never_create_actions() {
        let source = "call(); // fake(); next();\n/* comment; */ more();";
        assert_eq!(normalize(source), source);
    }

    #[test]
    fn multiline_data_braces_keep_inner_semicolons_opaque() {
        let source = "const row = infer struct {\na: 1; b: 2\n}; go();";
        assert_eq!(
            normalize(source),
            "const row = infer struct {\na: 1; b: 2\n};\n go();"
        );
    }

    #[test]
    fn multiline_calls_keep_inner_semicolons_opaque() {
        let source = "call(\n  first(); second(),\n  third()\n); finish();";
        assert_eq!(
            normalize(source),
            "call(\n  first(); second(),\n  third()\n);\n finish();"
        );
    }

    #[test]
    fn executable_body_lines_still_split_after_an_open_brace() {
        let source = "if ready {\nfirst(); second();\n}";
        assert_eq!(normalize(source), "if ready {\nfirst();\n second();\n}");
    }
}
