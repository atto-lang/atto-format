//! Expand compact brace-bodied control flow without touching expression braces.
//! This is a conservative lexical pass, not a substitute for the compiler.

use crate::{FormatError, LexState, inline_blocks, starts_word, statement_lines};

fn is_control_header(source: &str) -> bool {
    // Treat comments and quoted data as opaque when looking for a record
    // introducer. A literal containing "infer struct" is not a typed record
    // expression and must not suppress an executable control-body split.
    let mut state = LexState::default();
    let visible: String = crate::scan_visible(source, &mut state).into_iter().collect();
    let text = visible.trim_start();
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

fn begins_control_header(source: &str) -> bool {
    let mut state = LexState::default();
    let visible: String = crate::scan_visible(source, &mut state).into_iter().collect();
    let text = visible.trim_start();
    for keyword in [
        "if", "elif", "elseif", "else", "for", "while", "loop",
        "try", "catch", "finally",
    ] {
        if !starts_word(text, keyword) {
            continue;
        }
        let remainder = text[keyword.len()..].trim_start();
        if matches!(keyword, "else" | "loop" | "try" | "finally")
            && matches!(remainder.chars().next(), Some('(' | '.' | '['))
        {
            return false;
        }
        return true;
    }
    false
}

fn ambiguous_control_brace(line: &str, punctuation: &[(usize, char)]) -> bool {
    let (mut parens, mut brackets) = (0usize, 0usize);
    for &(at, ch) in punctuation {
        match ch {
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            '{' if parens == 0 && brackets == 0 => {
                let header = &line[..at];
                if !begins_control_header(header) {
                    return false;
                }
                let distinct_body_brace =
                    header.chars().last().is_some_and(char::is_whitespace)
                        && !header.trim_end().ends_with('=')
                        && is_control_header(header);
                return !distinct_body_brace;
            }
            _ => {}
        }
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
        // A comment-only suffix remains attached to the opening header.
        // A block comment followed by actual executable code, however,
        // must not shield that code from statement separation.
        if statement_lines::next_code_after_comments(after).is_none() {
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
    // A comment-only suffix belongs to the closing brace. If a block
    // comment is followed by executable code, move the whole suffix to a
    // separate physical line instead of leaving code beside the closer.
    if statement_lines::next_code_after_comments(trailing).is_none() {
        expanded.push_str(trailing);
    } else {
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
    let code = statement_lines::next_code_after_comments(suffix)?;
    if !["else", "elif", "elseif", "catch", "finally", "fi"]
        .iter()
        .any(|word| starts_word(code, word))
    {
        return None;
    }
    // Preserve intervening block comments with the closing brace; only
    // separate the following control arm.
    let comment = suffix[..suffix.len() - code.len()].trim_end();
    if comment.is_empty() {
        Some(format!("{}\n{}", &line[..first.0 + 1], code))
    } else {
        Some(format!("{} {}\n{}", &line[..first.0 + 1], comment, code))
    }
}

fn pass(source: &str) -> Result<String, FormatError> {
    let mut output = String::with_capacity(source.len());
    let mut lex = LexState::default();
    for (index, line) in source.split('\n').enumerate() {
        if index != 0 {
            output.push('\n');
        }
        let punctuation = inline_blocks::visible_punctuation(line, &mut lex);
        if ambiguous_control_brace(line, &punctuation) {
            return Err(FormatError::new(
                index + 1,
                "ambiguous compact control brace; separate data/record braces from the control body",
            ));
        }
        if let Some(split) = split_leading_closer(line, &punctuation) {
            output.push_str(&split);
        } else {
            output.push_str(&expand_one_line(line, &punctuation));
        }
    }
    Ok(output)
}

/// Repeat until nested compact blocks and statement-separated blocks settle.
/// Refuse pathologically nested source rather than returning non-idempotent
/// partially formatted output.
pub(crate) fn normalize(source: &str) -> Result<String, FormatError> {
    let mut current = statement_lines::normalize(source);
    for _ in 0..64 {
        let next = statement_lines::normalize(&pass(&current)?);
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
    fn quoted_data_and_comments_do_not_suppress_a_real_control_block() {
        let input = "if title == \"infer struct\" { first(); second(); }";
        let actual = normalize(input).unwrap();
        assert!(actual.contains("if title == \"infer struct\" {\n"));
        assert!(actual.contains("first();\n second();\n}"));
        assert_eq!(normalize(&actual).unwrap(), actual);

        let comment = "if ready /* infer struct */ { work(); }";
        let formatted = normalize(comment).unwrap();
        assert!(formatted.contains("if ready /* infer struct */ {\n"));
        assert!(formatted.contains("work();\n}"));
    }

    #[test]
    fn ambiguous_inline_type_brace_in_condition_fails_closed() {
        let source = "if value == infer struct {count: 1} { run(); }";
        let error = normalize(source).unwrap_err();
        assert_eq!(error.line(), 1);
        assert!(error.message().contains("ambiguous compact control brace"));
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
    fn ambiguous_attached_record_braces_fail_closed_instead_of_looking_formatted() {
        for source in [
            "if item == Point{x: 1} { run(); }",
            "if value == {x: 1} { run(); }",
            "if ready{ run(); }",
        ] {
            let error = normalize(source).unwrap_err();
            assert_eq!(error.line(), 1, "{source}");
            assert!(error.message().contains("ambiguous compact control brace"), "{source}");
        }
    }

    #[test]
    fn ambiguous_control_brace_reports_the_actual_source_line() {
        let source = "first();\nif ready{ second(); }\nthird();";
        let error = normalize(source).unwrap_err();
        assert_eq!(error.line(), 2);
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
    fn block_comments_before_actions_do_not_hide_closer_boundaries() {
        let input = "if ready { /* note only */\nwork();\n}";
        assert_eq!(normalize(input).unwrap(), input);

        let mixed = "if ready { /* body */ work();\nmore();\n}";
        let output = normalize(mixed).unwrap();
        assert!(output.starts_with("if ready {\n/* body */ work();"));
        assert_eq!(normalize(&output).unwrap(), output);

        let after = "if ready { work(); } /* note */ next();";
        let actual = normalize(after).unwrap();
        assert!(actual.contains("}\n/* note */ next();"));
        assert_eq!(normalize(&actual).unwrap(), actual);
    }

    #[test]
    fn leading_closer_comments_stay_with_the_closer_not_else_arm() {
        let input = "if ready { work();\n} /* branch note */ else { fallback(); }";
        let actual = normalize(input).unwrap();
        assert!(actual.contains("} /* branch note */\nelse {"), "{actual}");
        assert!(actual.contains("fallback();\n}"));
        assert_eq!(normalize(&actual).unwrap(), actual);
    }

    #[test]
    fn does_not_break_record_literals_across_lines() {
        let source = "const row = infer struct {\na: 1; b: 2\n}; go();";
        let actual = normalize(source).unwrap();
        assert_eq!(actual, "const row = infer struct {\na: 1; b: 2\n};\n go();");
        assert_eq!(normalize(&actual).unwrap(), actual);
    }
}
