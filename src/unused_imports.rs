//! Conservative unused-import commenting for the canonical Oreslang formatter.
//!
//! Imports may have initialization side effects. Retain intentionally
//! unreferenced imports with a preceding '// oresfmt: keep-import' marker.

use std::collections::HashSet;

#[derive(Clone, Debug)]
enum Kind {
    Word(String),
    Literal,
    Mark(char),
}

#[derive(Clone, Debug)]
struct Token {
    kind: Kind,
    start: usize,
    end: usize,
}

impl Token {
    fn word(&self, expected: &str) -> bool {
        matches!(&self.kind, Kind::Word(s) if s == expected)
    }

    fn mark(&self, expected: char) -> bool {
        matches!(&self.kind, Kind::Mark(ch) if *ch == expected)
    }

    fn name(&self) -> Option<&str> {
        if let Kind::Word(ref name) = self.kind {
            Some(name)
        } else {
            None
        }
    }
}

// Lex only enough source to recognize import bindings and identifier references.
// Strings, line comments, nested block comments, and #symbol literals are not
// references. Backtick templates fail closed because they may interpolate names.
fn tokens(source: &str) -> Option<Vec<Token>> {
    let mut chars = source.char_indices().peekable();
    let mut result = Vec::new();
    while let Some((start, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|&(_, c)| c == '/') {
            chars.next();
            while let Some(&(_, c)) = chars.peek() {
                if c == '\n' {
                    break;
                }
                chars.next();
            }
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|&(_, c)| c == '*') {
            chars.next();
            let mut depth = 1usize;
            while let Some((_, c)) = chars.next() {
                if c == '/' && chars.peek().is_some_and(|&(_, n)| n == '*') {
                    chars.next();
                    depth += 1;
                } else if c == '*' && chars.peek().is_some_and(|&(_, n)| n == '/') {
                    chars.next();
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            if depth != 0 {
                return None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' || ch == '\u{60}' {
            if ch == '\u{60}' {
                return None;
            }
            let mut escaped = false;
            let mut end = None;
            for (i, c) in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == ch {
                    end = Some(i + c.len_utf8());
                    break;
                }
            }
            result.push(Token {
                kind: Kind::Literal,
                start,
                end: end?,
            });
            continue;
        }
        if ch == '#' && chars.peek().is_some_and(|&(_, c)| is_word_start(c)) {
            // Oreslang #name is a Symbol literal, not a variable reference.
            while chars.peek().is_some_and(|&(_, c)| is_word_continue(c)) {
                chars.next();
            }
            continue;
        }
        if is_word_start(ch) {
            let mut end = start + ch.len_utf8();
            while let Some(&(i, c)) = chars.peek() {
                if !is_word_continue(c) {
                    break;
                }
                chars.next();
                end = i + c.len_utf8();
            }
            result.push(Token {
                kind: Kind::Word(source[start..end].to_string()),
                start,
                end,
            });
        } else {
            result.push(Token {
                kind: Kind::Mark(ch),
                start,
                end: start + ch.len_utf8(),
            });
        }
    }
    Some(result)
}

fn is_word_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_alphabetic()
}

fn is_word_continue(ch: char) -> bool {
    is_word_start(ch) || ch.is_numeric()
}

fn take_word<'a>(tokens: &'a [Token], at: &mut usize) -> Option<&'a str> {
    let name = tokens.get(*at)?.name()?;
    *at += 1;
    Some(name)
}

fn take_mark(tokens: &[Token], at: &mut usize, mark: char) -> Option<()> {
    if !tokens.get(*at)?.mark(mark) {
        return None;
    }
    *at += 1;
    Some(())
}

fn take_keyword(tokens: &[Token], at: &mut usize, keyword: &str) -> Option<()> {
    if !tokens.get(*at)?.word(keyword) {
        return None;
    }
    *at += 1;
    Some(())
}

// Returns locally exposed bindings and the token immediately after ';'.
// Unknown or malformed imports are left active rather than guessed at.
fn parse_import(tokens: &[Token], start: usize) -> Option<(Vec<String>, usize)> {
    let mut at = start;
    take_keyword(tokens, &mut at, "import")?;
    let mut bindings = Vec::new();
    if tokens.get(at)?.mark('*') {
        at += 1;
        take_keyword(tokens, &mut at, "as")?;
        bindings.push(take_word(tokens, &mut at)?.to_string());
    } else {
        let kind = take_word(tokens, &mut at)?;
        if !matches!(
            kind,
            "module" | "actor" | "class" | "fnc" | "interface" | "contract"
                | "trait" | "struct" | "type" | "types"
        ) {
            return None;
        }
        if tokens.get(at)?.mark('*') {
            at += 1;
            take_keyword(tokens, &mut at, "as")?;
            bindings.push(take_word(tokens, &mut at)?.to_string());
        } else {
            let close = if tokens.get(at)?.mark('{') {
                at += 1;
                Some('}')
            } else if tokens.get(at)?.mark('(') {
                at += 1;
                Some(')')
            } else {
                None
            };
            bindings.push(take_word(tokens, &mut at)?.to_string());
            while tokens.get(at).is_some_and(|t| t.mark(',')) {
                at += 1;
                bindings.push(take_word(tokens, &mut at)?.to_string());
            }
            if let Some(close) = close {
                take_mark(tokens, &mut at, close)?;
            }
            if tokens.get(at).is_some_and(|t| t.word("as")) {
                at += 1;
                if bindings.len() != 1 {
                    return None;
                }
                bindings = vec![take_word(tokens, &mut at)?.to_string()];
            }
        }
    }
    take_keyword(tokens, &mut at, "from")?;
    if !matches!(&tokens.get(at)?.kind, Kind::Literal) {
        return None;
    }
    at += 1;
    take_mark(tokens, &mut at, ';')?;
    Some((bindings, at))
}

struct Import {
    first: usize,
    last: usize,
    bindings: Vec<String>,
}

pub(super) fn comment_unused(source: &str) -> String {
    let Some(all) = tokens(source) else {
        return source.to_owned();
    };
    let mut at = 0;
    // Oreslang imports occur only in the source header, after an optional namespace.
    if all.first().is_some_and(|t| t.word("namespace")) {
        while at < all.len() && !all[at].mark(';') {
            at += 1;
        }
        if at == all.len() {
            return source.to_owned();
        }
        at += 1;
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut imports = Vec::new();
    while all.get(at).is_some_and(|t| t.word("import")) {
        let first_token = at;
        let Some((bindings, next)) = parse_import(&all, at) else {
            return source.to_owned();
        };
        let first_offset = all[first_token].start;
        let last_offset = all[next - 1].end;
        let first = source[..first_offset].bytes().filter(|b| *b == b'\n').count();
        let last = source[..last_offset].bytes().filter(|b| *b == b'\n').count();
        // Require an import to own complete physical lines, so comments cannot
        // swallow another declaration that happens to share its line.
        let beginning = source[..first_offset].rfind('\n').map_or(0, |i| i + 1);
        let line_end = source[last_offset..]
            .find('\n')
            .map_or(source.len(), |i| last_offset + i);
        let after = &source[last_offset..line_end];
        let standalone = source[beginning..first_offset].trim().is_empty()
            && (after.trim().is_empty() || after.trim_start().starts_with("//"));
        if standalone {
            imports.push(Import {
                first,
                last,
                bindings,
            });
        }
        at = next;
    }
    if imports.is_empty() {
        return source.to_owned();
    }

    let references: HashSet<&str> = all[at..].iter().filter_map(Token::name).collect();
    let mut comment = vec![false; lines.len()];
    for import in imports {
        let keep = lines[import.first..=import.last]
            .iter()
            .any(|s| s.contains("oresfmt: keep-import"))
            || (import.first > 0
                && lines[import.first - 1].contains("oresfmt: keep-import"));
        if !keep
            && !import
                .bindings
                .iter()
                .any(|name| references.contains(name.as_str()))
        {
            for flag in &mut comment[import.first..=import.last] {
                *flag = true;
            }
        }
    }
    if !comment.iter().any(|flag| *flag) {
        return source.to_owned();
    }
    let mut result = String::new();
    for (i, line) in lines.iter().enumerate() {
        if comment[i] {
            result.push_str("// ");
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}
