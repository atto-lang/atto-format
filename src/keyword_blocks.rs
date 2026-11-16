//! Expand single-line keyword conditionals before the indentation pass.
//! Comments, strings, and template literals remain opaque.

use crate::{LexState, is_ident_char};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Header,
    Body,
}

#[derive(Clone, Copy)]
struct Token<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

struct Lexed<'a> {
    tokens: Vec<Token<'a>>,
    line_comment: Option<usize>,
    block_comment_seen: bool,
}

fn lex_line<'a>(line: &'a str, state: &mut LexState) -> Lexed<'a> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut line_comment = None;
    let mut block_comment_seen = state.block_comment > 0;

    while let Some((start, ch)) = chars.next() {
        let next = chars.peek().map(|&(_, ch)| ch);
        if state.block_comment > 0 {
            // AttoLang block comments nest (Java Lexer and LLVM agree).
            if ch == '/' && next == Some('*') {
                state.block_comment += 1;
                chars.next();
                continue;
            }
            if ch == '*' && next == Some('/') {
                chars.next();
                state.block_comment -= 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            line_comment = Some(start);
            break;
        }
        if ch == '/' && next == Some('*') {
            chars.next();
            state.block_comment = 1;
            block_comment_seen = true;
            continue;
        }
        if matches!(ch, '"' | '\'' | '\x60') {
            let mut escape = false;
            let mut end = start + ch.len_utf8();
            state.quote = Some(ch);
            for (pos, next) in chars.by_ref() {
                end = pos + next.len_utf8();
                if escape {
                    escape = false;
                } else if next == '\\' {
                    escape = true;
                } else if next == ch {
                    state.quote = None;
                    break;
                }
            }
            tokens.push(Token {
                text: &line[start..end],
                start,
                end,
            });
            continue;
        }
        if ch.is_whitespace() {
            continue;
        }
        if is_ident_char(ch) {
            let mut end = start + ch.len_utf8();
            while let Some(&(pos, c)) = chars.peek() {
                if !is_ident_char(c) {
                    break;
                }
                chars.next();
                end = pos + c.len_utf8();
            }
            tokens.push(Token {
                text: &line[start..end],
                start,
                end,
            });
        } else {
            tokens.push(Token {
                text: &line[start..start + ch.len_utf8()],
                start,
                end: start + ch.len_utf8(),
            });
        }
    }

    Lexed {
        tokens,
        line_comment,
        block_comment_seen,
    }
}

fn statement_boundary(tokens: &[Token<'_>], index: usize) -> bool {
    index == 0
        || matches!(
            tokens[index - 1].text,
            ";" | "{" | "}" | "then" | "else" | "fi"
        )
}

// A contextual identifier used as a call, index, or member is not
// a control-flow delimiter.
fn is_keyword_delimiter(tokens: &[Token<'_>], index: usize) -> bool {
    let prev = index.checked_sub(1).and_then(|i| tokens.get(i));
    let next = tokens.get(index + 1);
    prev.is_none_or(|token| token.text != ".")
        && !next.is_some_and(|token| matches!(token.text, "(" | "[" | "."))
}

fn has_top_level_keyword_header(tokens: &[Token<'_>], start: usize) -> bool {
    let (mut parens, mut brackets) = (0usize, 0usize);

    for (offset, token) in tokens[start..].iter().enumerate() {
        match token.text {
            "(" => parens += 1,
            ")" => {
                if parens == 0 {
                    return false;
                }
                parens -= 1;
            }
            "[" => brackets += 1,
            "]" => {
                if brackets == 0 {
                    return false;
                }
                brackets -= 1;
            }
            "{" if parens == 0 && brackets == 0 => return false,
            "then" | "do" if parens == 0 && brackets == 0 => {
                let absolute = start + offset;
                if is_keyword_delimiter(tokens, absolute) {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HeaderTerminator {
    Keyword,
    Brace,
}

fn first_top_level_header_terminator(
    tokens: &[Token<'_>],
    start: usize,
) -> Option<HeaderTerminator> {
    let (mut parens, mut brackets) = (0usize, 0usize);

    for token in tokens.iter().skip(start) {
        match token.text {
            "(" => parens += 1,
            ")" => parens = parens.saturating_sub(1),
            "[" => brackets += 1,
            "]" => brackets = brackets.saturating_sub(1),
            "{" if parens == 0 && brackets == 0 => return Some(HeaderTerminator::Brace),
            "then" | "do" if parens == 0 && brackets == 0 => {
                return Some(HeaderTerminator::Keyword);
            }
            _ => {}
        }
    }

    None
}

fn collapse_multiline_keyword_headers(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut output = Vec::with_capacity(lines.len());
    let mut index = 0usize;

    while index < lines.len() {
        let first = lines[index];
        let mut first_state = LexState::default();
        let first_lexed = lex_line(first, &mut first_state);
        let first_tokens = &first_lexed.tokens;
        let header_start = first_tokens
            .first()
            .is_some_and(|token| matches!(token.text, "if" | "elif" | "elseif"));
        let first_terminator = first_top_level_header_terminator(first_tokens, 1);

        if !header_start
            || first_lexed.line_comment.is_some()
            || first_lexed.block_comment_seen
            || first_terminator.is_some()
        {
            output.push(first.to_string());
            index += 1;
            continue;
        }

        let mut joined = first.trim_end().to_string();
        let mut cursor = index + 1;
        let mut complete = None;
        let mut unsafe_to_join = false;

        while cursor < lines.len() {
            let next = lines[cursor];
            if next.trim().is_empty() {
                unsafe_to_join = true;
                break;
            }

            let mut next_state = LexState::default();
            let next_lexed = lex_line(next, &mut next_state);
            if next_lexed.line_comment.is_some() || next_lexed.block_comment_seen {
                unsafe_to_join = true;
                break;
            }

            joined.push(' ');
            joined.push_str(next.trim());

            let mut joined_state = LexState::default();
            let joined_lexed = lex_line(&joined, &mut joined_state);
            match first_top_level_header_terminator(&joined_lexed.tokens, 1) {
                Some(HeaderTerminator::Keyword) => {
                    complete = Some(cursor);
                    break;
                }
                Some(HeaderTerminator::Brace) => {
                    unsafe_to_join = true;
                    break;
                }
                _ => {}
            }

            cursor += 1;
        }

        if let Some(last) = complete.filter(|_| !unsafe_to_join) {
            output.push(joined);
            index = last + 1;
        } else {
            output.push(first.to_string());
            index += 1;
        }
    }

    output.join("\n")
}

fn inline_return_before(tokens: &[Token<'_>], index: usize) -> bool {
    let mut start = 0usize;
    for previous in (0..index).rev() {
        if matches!(
            tokens[previous].text,
            ";" | "{" | "}" | "then" | "else" | "fi"
        ) {
            start = previous + 1;
            break;
        }
    }
    let statement = &tokens[start..index];
    statement
        .first()
        .is_some_and(|token| matches!(token.text, "return" | "^"))
        && statement.len() >= 2
}

fn return_expression_is_multiline(tokens: &[Token<'_>], next: &str) -> bool {
    let expression_head = tokens.get(1).map(|token| token.text);
    let expression_tail = tokens.last().map(|token| token.text);

    matches!(expression_head, Some("match" | "select" | "switch" | "do"))
        || matches!(
            expression_tail,
            Some(
                "match"
                    | "select"
                    | "switch"
                    | "over"
                    | "from"
                    | "with"
                    | "as"
                    | "is"
                    | "await"
                    | "then"
                    | "do"
                    | "and"
                    | "or"
            )
        )
        || [".", "?.", "(", "[", "+", "-", "*", "/", "&&", "||", "^^", "|>", "<|", "<<", ">>"]
            .iter()
            .any(|prefix| next.starts_with(prefix))
        || next.starts_with("/*")
        || ["on", "when", "case", "default", "as", "is", "with", "over"]
            .iter()
            .any(|word| crate::starts_word(next, word))
        || ["and", "or", "pipe", "rpipe"]
            .into_iter()
            .any(|word| crate::starts_word_operator_continuation(next, word))
}

fn split_keyword_blocks(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut lex = LexState::default();
    let mut frames: Vec<Phase> = Vec::new();

    for (line_number, line) in source.split('\n').enumerate() {
        if line_number != 0 {
            output.push('\n');
        }
        let scanned = lex_line(line, &mut lex);
        let tokens = scanned.tokens;
        let mut cuts: Vec<usize> = Vec::new();
        // A keyword if may have opened on a previous line. Every logical
        // statement in its body still needs its own line.
        let mut saw_keyword_arm = !frames.is_empty();
        let (mut parens, mut brackets, mut braces) = (0usize, 0usize, 0usize);

        for (index, token) in tokens.iter().enumerate() {
            match token.text {
                "(" => parens += 1,
                ")" => parens = parens.saturating_sub(1),
                "[" => brackets += 1,
                "]" => brackets = brackets.saturating_sub(1),
                "{" => braces += 1,
                "}" => braces = braces.saturating_sub(1),
                _ => {}
            }
            if parens != 0 || brackets != 0 || braces != 0 {
                continue;
            }

            match token.text {
                "if" if statement_boundary(&tokens, index) => {
                    // Braced if/elif arms are a separate grammatical form.
                    // Require an actual keyword header terminator to opt in.
                    let has_keyword_header = has_top_level_keyword_header(&tokens, index + 1);
                    if has_keyword_header && (index == 0 || tokens[index - 1].text != "else") {
                        frames.push(Phase::Header);
                        if index > 0 {
                            cuts.push(token.start);
                        }
                    }
                }
                "then" | "do"
                    if frames.last() == Some(&Phase::Header)
                        && index > 0
                        && is_keyword_delimiter(&tokens, index) =>
                {
                    if let Some(frame) = frames.last_mut() {
                        *frame = Phase::Body;
                    }
                    saw_keyword_arm = true;
                    if index + 1 < tokens.len() {
                        cuts.push(token.end);
                    }
                }
                "elif" | "elseif"
                    if !frames.is_empty()
                        && statement_boundary(&tokens, index)
                        && is_keyword_delimiter(&tokens, index) =>
                {
                    saw_keyword_arm = true;
                    cuts.push(token.start);
                    if let Some(frame) = frames.last_mut() {
                        *frame = Phase::Header;
                    }
                }
                "else"
                    if !frames.is_empty()
                        && statement_boundary(&tokens, index)
                        && is_keyword_delimiter(&tokens, index) =>
                {
                    saw_keyword_arm = true;
                    cuts.push(token.start);
                    if let Some(frame) = frames.last_mut() {
                        *frame = if tokens.get(index + 1).is_some_and(|t| t.text == "if") {
                            Phase::Header
                        } else {
                            Phase::Body
                        };
                    }
                    if frames.last() == Some(&Phase::Body) && index + 1 < tokens.len() {
                        cuts.push(token.end);
                    }
                }
                "fi" if frames.last() == Some(&Phase::Body)
                    && is_keyword_delimiter(&tokens, index)
                    && (statement_boundary(&tokens, index)
                        || inline_return_before(&tokens, index)
                        || tokens
                            .get(index.wrapping_sub(1))
                            .is_some_and(|t| t.text == ")")) =>
                {
                    saw_keyword_arm = true;
                    cuts.push(token.start);
                    frames.pop();
                    if index + 1 < tokens.len() && tokens[index + 1].text != ";" {
                        cuts.push(token.end);
                    }
                }
                ";" if saw_keyword_arm
                    && frames.last() == Some(&Phase::Body)
                    && index + 1 < tokens.len()
                    && !matches!(tokens[index + 1].text, "do" | "then") =>
                {
                    cuts.push(token.end);
                }
                _ => {}
            }
        }

        cuts.sort_unstable();
        cuts.dedup();
        let mut last = 0;
        for cut in cuts {
            if line[cut..].trim().is_empty() {
                continue;
            }
            if line[last..cut].trim().is_empty() {
                // A preceding semicolon already inserted the newline.
                // Discard the whitespace between that terminator and fi.
                last = cut;
                continue;
            }
            output.push_str(line[last..cut].trim_end());
            output.push('\n');
            last = cut;
        }
        output.push_str(&line[last..]);
    }
    output
}

fn terminate_simple_returns(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut lex = LexState::default();
    let lines: Vec<&str> = source.split('\n').collect();

    for (index, line) in lines.iter().enumerate() {
        if index != 0 {
            output.push('\n');
        }
        let initial = lex;
        let scanned = lex_line(line, &mut lex);
        let tokens = &scanned.tokens;
        let code_end = scanned.line_comment.unwrap_or(line.len());
        let code = line[..code_end].trim_end();

        // Add a terminator only when the return or its ^ shorthand is
        // definitely complete on this line. Bare returns are complete.
        // Expression returns
        // fail closed around multiline/block syntax so formatting cannot
        // silently terminate a future match/select/continuation early.
        let is_return = initial.block_comment == 0
            && !scanned.block_comment_seen
            && tokens
                .first()
                .is_some_and(|token| matches!(token.text, "return" | "^"))
            // The merged logical-XOR token is ^^; spaced ^ ^ is also
            // rejected source, not a return statement to auto-terminate.
            && !code.starts_with("^^")
            && !tokens.get(1).is_some_and(|token| token.text == "^")
            && !code.ends_with(';');
        let bare_return = is_return && tokens.len() == 1;
        let simple_expression_return = is_return
            && tokens.len() >= 2
            && !code.ends_with('{')
            && !code.ends_with('}')
            && !code.ends_with(',')
            && !code.ends_with('.')
            && !code.ends_with(':')
            && !code.ends_with('?')
            && !code.ends_with('+')
            && !code.ends_with('-')
            && !code.ends_with('*')
            && !code.ends_with('/')
            && !code.ends_with('=')
            && !code.ends_with('>')
            && !code.ends_with('<')
            && !code.ends_with('|')
            && !code.ends_with('&')
            && !tokens
                .last()
                .is_some_and(|token| matches!(token.text, "and" | "or"))
            && !code.contains("->")
            && !code.contains("=>")
            && !tokens
                .iter()
                .any(|token| token.text == "{" || token.text == "}");

        let (mut parens, mut brackets) = (0isize, 0isize);
        let mut invalid = false;
        for token in tokens {
            match token.text {
                "(" => parens += 1,
                ")" => parens -= 1,
                "[" => brackets += 1,
                "]" => brackets -= 1,
                _ => {}
            }
            invalid |= parens < 0 || brackets < 0;
        }
        let next = lines
            .iter()
            .skip(index + 1)
            .map(|line| line.trim_start())
            .find(|line| !line.is_empty() && !line.starts_with("//"))
            .unwrap_or("");
        let definitely_complete = bare_return
            || (simple_expression_return
                && !return_expression_is_multiline(tokens, next)
                && !invalid
                && parens == 0
                && brackets == 0);

        if definitely_complete {
            let at = code.len();
            output.push_str(&line[..at]);
            output.push(';');
            output.push_str(&line[at..]);
        } else {
            output.push_str(line);
        }
    }
    output
}

/// Break compact conditional arms and safely terminate full-line returns.
/// `for ... do body` / `while ... do body` with the body continuing on later
/// lines (closed by a `done` on another line) is valid source. Move the body
/// onto its own line so the `do` header opens a block like any other loop.
/// A complete one-line `... do body; done` loop is left as written.
fn split_compact_loop_headers(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut lex = LexState::default();
    for (line_number, line) in source.split('\n').enumerate() {
        if line_number != 0 {
            output.push('\n');
        }
        let tokens = lex_line(line, &mut lex).tokens;
        let starts_loop = tokens
            .first()
            .is_some_and(|token| matches!(token.text, "for" | "while"));
        let mut cut = None;
        if starts_loop {
            let (mut parens, mut brackets, mut braces) = (0usize, 0usize, 0usize);
            for (index, token) in tokens.iter().enumerate() {
                match token.text {
                    "(" => parens += 1,
                    ")" => parens = parens.saturating_sub(1),
                    "[" => brackets += 1,
                    "]" => brackets = brackets.saturating_sub(1),
                    "{" => braces += 1,
                    "}" => braces = braces.saturating_sub(1),
                    _ => {}
                }
                if parens != 0 || brackets != 0 || braces != 0 {
                    continue;
                }
                if token.text == "{" {
                    break; // a braced loop body, not a `do` loop
                }
                if token.text == "do" && index > 0 && is_keyword_delimiter(&tokens, index) {
                    let has_body = index + 1 < tokens.len();
                    let closes_here = tokens[index + 1..].iter().any(|t| t.text == "done");
                    if has_body && !closes_here {
                        cut = Some(token.end);
                    }
                    break;
                }
            }
        }
        match cut {
            Some(at) => {
                output.push_str(line[..at].trim_end());
                output.push('\n');
                output.push_str(line[at..].trim_start());
            }
            None => output.push_str(line),
        }
    }
    output
}

pub(crate) fn normalize(source: &str) -> String {
    let collapsed = collapse_multiline_keyword_headers(source);
    let loops = split_compact_loop_headers(&collapsed);
    terminate_simple_returns(&split_keyword_blocks(&loops))
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn expands_multiline_keyword_if_header_before_compact_body() {
        assert_eq!(
            normalize("if ready &&\n   permitted; do return false; fi"),
            "if ready && permitted; do\n return false;\nfi"
        );
    }

    #[test]
    fn multiline_braced_if_is_not_collapsed_into_keyword_form() {
        let source = "if ready &&\n   permitted\n{\nwork();\n}";
        assert_eq!(normalize(source), source);
    }

    #[test]
    fn expands_keyword_if_with_terminator() {
        assert_eq!(
            normalize("if self.built; then return Err(\"builder already consumed\"); fi"),
            "if self.built; then\n return Err(\"builder already consumed\");\nfi"
        );
    }

    #[test]
    fn inserts_return_terminator() {
        assert_eq!(
            normalize("if done then return Err(\"not ready\") fi"),
            "if done then\n return Err(\"not ready\");\nfi"
        );
    }

    #[test]
    fn comments_and_literals_remain_opaque() {
        let line = "if ready; then return \"else; fi; then\"; fi // then fi";
        assert_eq!(
            normalize(line),
            "if ready; then\n return \"else; fi; then\";\nfi // then fi"
        );
    }

    #[test]
    fn contextual_then_and_do_inside_braced_conditions_do_not_open_keyword_blocks() {
        for source in [
            "if predicate.then() {\nwork();\n}\nfi",
            "if predicate.do() {\nwork();\n}\nfi",
            "if predicate(then) {\nwork();\n}\nfi",
            "if predicate(do) {\nwork();\n}\nfi",
        ] {
            assert_eq!(normalize(source), source);
        }
    }

    #[test]
    fn top_level_then_after_nested_contextual_tokens_is_still_recognized() {
        assert_eq!(
            normalize("if predicate(then) && obj.do(); then return 1 fi"),
            "if predicate(then) && obj.do(); then\n return 1;\nfi"
        );
    }

    #[test]
    fn closes_inline_if_after_literal_and_identifier_returns_without_semicolons() {
        assert_eq!(
            normalize("if ready then return 1 fi"),
            "if ready then\n return 1;\nfi"
        );
        assert_eq!(
            normalize("if ready then return value fi"),
            "if ready then\n return value;\nfi"
        );
    }

    #[test]
    fn fi_as_return_operand_is_not_confused_with_the_structural_closer() {
        assert_eq!(
            normalize("if ready; then return fi; fi"),
            "if ready; then\n return fi;\nfi"
        );
    }

    #[test]
    fn canonicalizes_bare_return_and_preserves_line_comment() {
        assert_eq!(normalize("return"), "return;");
        assert_eq!(normalize("return // done"), "return; // done");
        assert_eq!(normalize("^"), "^;");
        assert_eq!(normalize("^ // done"), "^; // done");
        assert_eq!(normalize("^ 42"), "^ 42;");
        assert_eq!(normalize("^ value // done"), "^ value; // done");
    }

    #[test]
    fn caret_return_in_compact_keyword_if_splits_before_fi() {
        assert_eq!(normalize("if ready then ^ 42 fi"), "if ready then\n ^ 42;\nfi");
    }

    #[test]
    fn contextual_operator_identifier_after_return_is_a_new_statement() {
        for op in ["pipe", "and", "or"] {
            for keyword in ["return", "^"] {
                let source = format!("{keyword} result\n{op} = 3;");
                let expected = format!("{keyword} result;\n{op} = 3;");
                assert_eq!(normalize(&source), expected, "{source}");
            }
        }
        assert_eq!(
            normalize("return source\npipe spawn Consumer();"),
            "return source\npipe spawn Consumer();"
        );
    }

    #[test]
    fn caret_return_keeps_multiline_directional_and_actor_pipelines_open() {
        for source in [
            "^ spawn Producer()\npipe spawn Consumer();",
            "^ future\n|> transform;",
            "^ source\n>> val result;",
            "^ sink\n<< payload;",
            "^ pending\n<| transform_result;",
            "return sink\nrpipe source;",
            "return future\n|> transform\n<| finalizer;",
        ] {
            let output = normalize(source);
            assert!(
                !output.lines().next().unwrap().ends_with(';'),
                "shorthand return was terminated before the next operator: {output}"
            );
        }
        assert_eq!(normalize("\"^ 42\" // ^ 3"), "\"^ 42\" // ^ 3");
        assert_eq!(normalize("^^ value"), "^^ value");
        assert_eq!(normalize("^ ^ value"), "^ ^ value");
        assert_eq!(normalize("^ 42 ^^ 1"), "^ 42 ^^ 1;");
    }

    #[test]
    fn refuses_to_terminate_multiline_expression_introducers() {
        assert_eq!(
            normalize("return match value over\non _ -> 1"),
            "return match value over\non _ -> 1"
        );
        assert_eq!(
            normalize("return select from plan"),
            "return select from plan"
        );
        assert_eq!(
            normalize("return value\n  .transform()"),
            "return value\n  .transform()"
        );
        assert_eq!(
            normalize("return value\n  as Result"),
            "return value\n  as Result"
        );
    }

    #[test]
    fn still_terminates_obviously_complete_async_and_cast_returns() {
        assert_eq!(normalize("return await task()"), "return await task();");
        assert_eq!(
            normalize("return value as Result"),
            "return value as Result;"
        );
        assert_eq!(normalize("return obj.match()"), "return obj.match();");
    }
}
