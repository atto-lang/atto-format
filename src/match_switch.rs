//! Canonicalize optional separators between completed switch/match arms.
//! Only remove punctuation immediately after a verified arm body; quoted and
//! commented characters, and commas inside case alternatives, are untouched.

use std::collections::BTreeSet;
use crate::select::{matching_brace, tokens, Token};

#[derive(Clone, Copy)]
enum Kind { Switch, Match }

fn arm_starts(kind: Kind, text: &str) -> bool {
    match kind {
        Kind::Switch => matches!(text, "case" | "default" | "_"),
        Kind::Match => text == "on",
    }
}

fn outer_open(tokens: &[Token<'_>], at: usize, kind: Kind) -> Option<usize> {
    let mut parens = 0usize;
    let mut saw_over = false;
    for i in at + 1..tokens.len() {
        match tokens[i].text {
            "(" => parens += 1,
            ")" => parens = parens.saturating_sub(1),
            "over" if parens == 0 => saw_over = true,
            "{" if parens == 0 => {
                if matches!(kind, Kind::Switch) || saw_over {
                    return Some(i);
                }
                return None;
            }
            "end" | ";" if parens == 0 => return None,
            _ => {}
        }
    }
    None
}

pub(crate) fn normalize(source: &str) -> String {
    let lexed = tokens(source);
    let mut removed = BTreeSet::<usize>::new();

    for (i, token) in lexed.iter().enumerate() {
        let kind = match token.text {
            "switch" => Kind::Switch,
            "match" => Kind::Match,
            _ => continue,
        };
        // Ignore calls and member names that merely contain these words.
        if i > 0 && lexed[i - 1].text == "." { continue; }
        let Some(open) = outer_open(&lexed, i, kind) else { continue; };
        let Some(close) = matching_brace(&lexed, open) else { continue; };
        let mut arm = open + 1;
        while arm < close {
            if !arm_starts(kind, lexed[arm].text) {
                arm += 1;
                continue;
            }
            // Scan only within the block, skipping parenthesized tuple
            // bindings and switch scrutinee alternatives.
            let mut parens = 0usize;
            let mut brackets = 0usize;
            let mut body = arm + 1;
            while body < close {
                match lexed[body].text {
                    "(" => parens += 1,
                    ")" => parens = parens.saturating_sub(1),
                    "[" => brackets += 1,
                    "]" => brackets = brackets.saturating_sub(1),
                    "{" if parens == 0 && brackets == 0 => break,
                    _ => {}
                }
                body += 1;
            }
            if body == close { break; }
            let Some(end) = matching_brace(&lexed, body).filter(|end| *end < close) else {
                break;
            };
            let mut next = end + 1;
            while next < close && matches!(lexed[next].text, "," | ";") {
                // Delete only visible delimiter bytes (comments and all
                // surrounding text stay exactly where they were).
                for pos in lexed[next].start..lexed[next].end { removed.insert(pos); }
                next += 1;
            }
            arm = next;
        }
    }

    if removed.is_empty() { return source.to_owned(); }
    source.char_indices()
        .filter_map(|(pos, ch)| (!removed.contains(&pos)).then_some(ch))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn strips_only_arm_separators_not_case_alternatives() {
        let input = "switch (v) {\n  _ { return 0; },\n  case \"foo\", \"bar\" { return 1; };\n}\n";
        let output = normalize(input);
        assert!(output.contains("_ { return 0; }\n"));
        assert!(output.contains("case \"foo\", \"bar\" { return 1; }\n"));
        assert_eq!(normalize(&output), output);
    }

    #[test]
    fn leaves_unrelated_braces_comments_and_literals_intact() {
        let input = "fnc f() {\n  let s = \"case } , ;\";\n  select { default: { return; }, }\n  match msg over {\n    on Tag as (a, b) { log(a, b); }, // a comment\n    on _ { return; };\n  }\n}\n";
        let output = normalize(input);
        assert!(output.contains("select { default: { return; }, }"));
        assert!(output.contains("on Tag as (a, b) { log(a, b); } // a comment"));
        assert!(output.contains("on _ { return; }"));
        assert!(output.contains("\"case } , ;\""));
    }
}
