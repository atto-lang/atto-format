//! Expand compact brace-bodied control flow without touching expression braces.
//! This is a conservative lexical pass, not a substitute for the compiler.

use crate::{FormatError, LexState, inline_blocks, starts_word, statement_lines};

fn is_control_header(source: &str) -> bool {
    let text = source.trim_start();
    for keyword in [
        "if", "elif", "elseif", "else", "for", "while", "loop",
        "try", "catch", "finally",
    ] {
        if !starts_word(text, keyword) {
            continue;
        }
        let remainder = text[keyword.len()..].trim_start();
        // 'else()', 'loop()' etc. can be contextual call syntax.
        if matches!(keyword, "else" | "loop" | "try" | "finally")
            && matches!(remainder.chars().next(), Some('(' | '.' | '['))
        {
            return false;
        }
        // Fail closed for expressions with top-level inline record/type
        // introducers: their first brace is data, not a control-body brace.
        // The compiler and future AST-aware formatter must disambiguate.
        if text.contains("infer struct")
            || text.contains("infer record")
            || text.contains("obj{")
            || text.contains("new struct")
        {
            return false;
        }
        // Keyword-bodied conditionals are normalized by keyword_blocks,
        // not by an independent brace heuristic.
        if matches!(keyword, "if" | "elif" | "elseif")
            && (text.ends_with(" then") || text.ends_with(" do"))
        {
            return false;
        }
        return true;
    }
    false
}

fn expand_one_line(line: &str, punctuation: &[(usize, char)]) -> String {
    let (mut parens, mut brackets) = (0usize, 0usize);
    let mut opening = None;

    for (index, &(_, ch)) in punctuation.iter().enumerate() {
        match ch {
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            '{' if parens == 0 && brackets == 0 => {
                opening = Some(index);
                break;
            }
            _ => {}
        }
    }
    let Some(opening) = opening else {
        return line.to_string();
    };
    let open_at = punctuation[opening].0;
    if !is_control_header(&line[..open_at]) {
        return line.to_string();
    }

    let mut depth = 0usize;
    let mut close_at = None;
    for &(at, ch) in punctuation.iter().skip(opening) {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close_at = Some(at);
                    break;
                }
            }
            _ => {}
        }
    }
    // Do not reformat incomplete or cross-line braces heuristically.
    let Some(close_at) = close_at else {
        return line.to_string();
    };

    let body = &line[open_at + 1..close_at];
    let trailing = &line[close_at + 1..];
    let mut expanded = String::with_capacity(line.len() + 3);
    expanded.push_str(&line[..open_at + 1]);
    expanded.push('\n');
    if !body.trim().is_empty() {
        expanded.push_str(body.trim_end());
        expanded.push('\n');
    }
    expanded.push('}');
    // A line comment belongs to the closing brace and must not move.
    if trailing.trim_start().starts_with("//") || trailing.trim_start().starts_with("/*") {
        expanded.push_str(trailing);
    } else if !trailing.trim().is_empty() {
        expanded.push('\n');
        expanded.push_str(trailing.trim_start());
    }
    expanded
}

fn pass(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut lex = LexState::default();
    for (index, line) in source.split('\n').enumerate() {
        if index != 0 {
            output.push('\n');
        }
        let punctuation = inline_blocks::visible_punctuation(line, &mut lex);
        output.push_str(&expand_one_line(line, &punctuation));
    }
    output
}

/// Repeat until nested compact blocks and statement-separated blocks settle.
/// Refuse pathologically nested source rather than returning non-idempotent
/// partially formatted output.
pub(crate) fn normalize(source: &str) -> Result<String, FormatError> {
    let mut current = statement_lines::normalize(source);
    for _ in 0..64 {
        let next = statement_lines::normalize(&pass(&current));
        if next == current {
            return Ok(current);
        }
        current = next;
    }
    Err(FormatError::new(
        1,
        "compact control-flow nesting exceeds the formatter expansion budget",
    ))
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn expands_nested_and_chained_braced_conditions() {
        let source = "if outer { first(); if inner { second(); } } else { third(); } fi";
        let formatted = normalize(source).unwrap();
        assert!(formatted.contains("if outer {\n"));
        assert!(formatted.contains("first();\n if inner {\n"));
        assert!(formatted.contains("second();\n}"));
        assert!(formatted.contains("else {\n"));
        assert!(formatted.contains("third();\n}"));
        assert_eq!(normalize(&formatted).unwrap(), formatted);
    }

    #[test]
    fn preserves_expression_record_braces_and_comments() {
        let input = "const value = infer struct {label: \"{;}\"}; go();";
        assert_eq!(
            normalize(input).unwrap(),
            "const value = infer struct {label: \"{;}\"};\n go();"
        );
        let source = "if ready { work(\"} ;\"); } // comment; }";
        assert_eq!(
            normalize(source).unwrap(),
            "if ready {\n work(\"} ;\");\n} // comment; }"
        );
    }

    #[test]
    fn ambiguous_inline_type_brace_in_condition_is_not_treated_as_body() {
        let source = "if value == infer struct {count: 1} { run(); }";
        assert_eq!(normalize(source).unwrap(), source);
    }

    #[test]
    fn ignores_incomplete_and_multiline_brace_bodies() {
        let source = "if ready {\nwork();\n}\nfi";
        assert_eq!(normalize(source).unwrap(), source);
    }
}
