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
    let header = &line[..open_at];
    // Attached braces (Point{x: 1}, ready{...}) are ambiguous in an
    // evolving grammar. Likewise an opening brace after an expression
    // operator may be a data value, not a control body. Require a distinct
    // brace token rather than rewriting a possibly typed record literal.
    if header.chars().last().is_some_and(|ch| !ch.is_whitespace())
        || header.trim_end().ends_with('=')
        || !is_control_header(header)
    {
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
    // The opening brace is unambiguously a control-body delimiter, even
    // when its matching closer is on a later physical line. Split only this
    // boundary; leave the remainder and nested lexical structure to later
    // passes. Do not manufacture a blank line for an already-open header.
    let Some(close_at) = close_at else {
        let after = &line[open_at + 1..];
        // Keep a trailing line comment attached to the opening header.
        // Moving it onto a fresh line would change which source line owns
        // the comment and create unnecessary layout churn.
        if after.trim().is_empty() || after.trim_start().starts_with("//") {
            return line.to_string();
        }
        return format!("{}\n{}", &line[..open_at + 1], after.trim_start());
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

// Control closers on a preceding line may be joined to a following arm:
// `} else {`, `} catch {`, `} finally {` or `} fi`. A data brace
// cannot legally introduce any of these continuations, so this remains
// syntax-directed and does not depend on tracking a guessed brace owner.
fn split_leading_closer(line: &str, punctuation: &[(usize, char)]) -> Option<String> {
    let first = punctuation.first()?;
    if first.1 != '}' || !line[..first.0].trim().is_empty() {
        return None;
    }
    let suffix = line[first.0 + 1..].trim_start();
    if !["else", "elif", "elseif", "catch", "finally", "fi"]
        .iter()
        .any(|word| starts_word(suffix, word))
    {
        return None;
    }
    Some(format!("{}\n{}", &line[..first.0 + 1], suffix))
}

fn pass(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut lex = LexState::default();
    for (index, line) in source.split('\n').enumerate() {
        if index != 0 {
            output.push('\n');
        }
        let punctuation = inline_blocks::visible_punctuation(line, &mut lex);
        if let Some(split) = split_leading_closer(line, &punctuation) {
            output.push_str(&split);
        } else {
            output.push_str(&expand_one_line(line, &punctuation));
        }
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
    fn opens_multiline_control_bodies_and_separates_joined_else() {
        let source = "if ready { first(); second();\nthird(); fourth();\n} else { fallback(); }";
        let actual = normalize(source).unwrap();
        assert!(actual.contains("if ready {\nfirst();\n second();"));
        assert!(actual.contains("third();\n fourth();"));
        assert!(actual.contains("}\nelse {\nfallback();\n}"));
        assert_eq!(normalize(&actual).unwrap(), actual);
    }

    #[test]
    fn ambiguous_attached_record_braces_are_never_control_openers() {
        for source in [
            "if item == Point{x: 1} { run(); }",
            "if value == {x: 1} { run(); }",
            "if ready{ run(); }",
        ] {
            assert_eq!(normalize(source).unwrap(), source);
        }
    }

    #[test]
    fn keeps_already_separated_multiline_control_bodies_stable() {
        let source = "if ready {\nwork();\n}\nfi";
        assert_eq!(normalize(source).unwrap(), source);
    }

    #[test]
    fn multiline_control_keeps_opening_line_comment_attached() {
        let input = "if ready { // comment contains }; else and ;\nwork(); next();\n}";
        let expected = "if ready { // comment contains }; else and ;\nwork();\n next();\n}";
        assert_eq!(normalize(input).unwrap(), expected);
        assert_eq!(normalize(expected).unwrap(), expected);
    }

    #[test]
    fn multiline_control_still_splits_real_code_after_opening_brace() {
        let input = "if ready { /* comment */ work(); next();\nmore();\n}";
        let formatted = normalize(input).unwrap();
        assert!(formatted.contains("if ready {\n"));
        assert!(formatted.contains("/* comment */ work();\n next();"));
        assert_eq!(normalize(&formatted).unwrap(), formatted);
    }

    #[test]
    fn does_not_break_record_literals_across_lines() {
        let source = "const row = infer struct {\na: 1; b: 2\n}; go();";
        let actual = normalize(source).unwrap();
        assert_eq!(actual, "const row = infer struct {\na: 1; b: 2\n};\n go();");
        assert_eq!(normalize(&actual).unwrap(), actual);
    }
}
