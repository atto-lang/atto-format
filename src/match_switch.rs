//! Canonicalize optional separators between completed switch/match arms.
//! Only remove punctuation immediately after a verified arm body; quoted and
//! commented characters, and commas inside case alternatives, are untouched.

use std::collections::{BTreeMap, BTreeSet};
use crate::select::{matching_brace, tokens, Token};

#[derive(Clone, Copy)]
enum Kind { Switch, Match }

fn arm_starts(kind: Kind, text: &str) -> bool {
    match kind {
        Kind::Switch => matches!(text, "case" | "default" | "_"),
        Kind::Match => matches!(text, "on" | "default"),
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
    let mut inserted = BTreeMap::<usize, &'static str>::new();

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
            // Rewrite only verified catch-all arm heads, never tuple discards.
            let start = arm;
            if matches!(kind, Kind::Switch) && lexed[start].text == "_" {
                for pos in lexed[start].start..lexed[start].end { removed.insert(pos); }
                inserted.insert(lexed[start].start, "default");
            }
            if matches!(kind, Kind::Match) && lexed[start].text == "on"
                && start + 1 < body && matches!(lexed[start + 1].text, "_" | "default")
                // The tokenizer ignores comments. Keep intervening trivia
                // exactly as authored, including line and block comments.
            {
                for pos in lexed[start].start..lexed[start].end { removed.insert(pos); }
                if lexed[start + 1].text == "_" {
                    for pos in lexed[start + 1].start..lexed[start + 1].end { removed.insert(pos); }
                    inserted.insert(lexed[start + 1].start, "default");
                }
            }
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
    let mut result = String::with_capacity(source.len() + inserted.len() * 6);
    for (pos, ch) in source.char_indices() {
        if let Some(replacement) = inserted.get(&pos) { result.push_str(replacement); }
        if !removed.contains(&pos) { result.push(ch); }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn strips_only_arm_separators_not_case_alternatives() {
        let input = "switch (v) {\n  _ { return 0; },\n  case \"foo\", \"bar\" { return 1; };\n}\n";
        let output = normalize(input);
        assert!(output.contains("default { return 0; }\n"));
        assert!(output.contains("case \"foo\", \"bar\" { return 1; }\n"));
        assert_eq!(normalize(&output), output);
    }

    #[test]
    fn leaves_unrelated_braces_comments_and_literals_intact() {
        let input = "fnc f() {\n  let s = \"case } , ;\";\n  select { default: { return; }, }\n  match msg over {\n    on Tag as (a, b) { log(a, b); }, // a comment\n    on _ { return; };\n  }\n}\n";
        let output = normalize(input);
        assert!(output.contains("select { default: { return; }, }"));
        assert!(output.contains("on Tag as (a, b) { log(a, b); } // a comment"));
        assert!(output.contains("default { return; }"));
        assert!(output.contains("\"case } , ;\""));
    }

    #[test]
    fn canonicalizes_only_catchall_head_tokens() {
        let source = r#"switch v {
  _ { return "star"; }
  case "foo", "bar" { return "baz"; }
}
match msg over {
  on _ as remaining { handle(remaining); }
  on Thing.Tuple as (_, item) { handle(item); }
}
match msg over {
  on default { return; }
}
"#;
        let output = normalize(source);
        assert!(output.contains("default { return \"star\"; }"), "{output}");
        assert!(output.contains("default as remaining { handle(remaining); }"), "{output}");
        assert!(output.contains("on Thing.Tuple as (_, item) { handle(item); }"), "{output}");
        assert!(output.contains("default { return; }"), "{output}");
        assert!(!output.contains("on default"), "{output}");
        assert!(!output.contains("on _ as"), "{output}");
        assert_eq!(normalize(&output), output);
    }

    #[test]
    fn preserves_comments_between_on_and_catchall_alias() {
        let source = r#"match v over {
  on /*why*/ _ as item { handle(item); }
  on 7 { handle(7); }
}
"#;
        let output = normalize(source);
        assert!(output.contains("/*why*/ default as item"), "{output}");
        assert!(!output.contains("on /*why*/ _"), "{output}");
        assert_eq!(normalize(&output), output);
    }

    #[test]
    fn ignores_underscores_in_unrelated_syntax_and_strings() {
        let source = r#"fnc f() {
  const [x, _] = [1, 2];
  val quoted = "on _ { }";
  return quoted;
}"#;
        assert_eq!(normalize(source), source);
    }

}
