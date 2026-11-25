//! Golden and property tests over `tests/fixtures/properties/*.atto`.
//!
//! Every input fixture was checked with Java `OresMain --check` (and LLVM
//! `oreslang-llvmc` where noted in the fixture comments of the PR evidence),
//! before and after formatting, and Java run output was identical. That is
//! audit evidence recorded outside this test; these tests are host-only.
//!
//! For each fixture `NAME.atto` with golden `NAME.expected.atto`:
//! - formatting yields exactly the golden, and the golden is a fixed point;
//! - re-indenting, tab indentation, CRLF line endings and trailing
//!   whitespace do not change the canonical output;
//! - every char-boundary prefix formats or is rejected without panicking;
//! - token preservation: comments, string literals and numbers are kept
//!   byte-for-byte and in order, and identifiers are kept as a multiset
//!   except for the keywords the formatter is documented to rewrite.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use oreslang_format::format_source;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/properties")
}

fn fixtures() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(fixture_dir()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".atto") || name.ends_with(".expected.atto") {
            continue;
        }
        let expected = path.with_file_name(name.replace(".atto", ".expected.atto"));
        out.push((
            name,
            fs::read_to_string(&path).unwrap(),
            fs::read_to_string(&expected)
                .unwrap_or_else(|_| panic!("missing golden {}", expected.display())),
        ));
    }
    out.sort();
    assert!(
        out.len() >= 9,
        "fixture corpus unexpectedly small: {}",
        out.len()
    );
    out
}

fn fmt(name: &str, source: &str) -> String {
    format_source(source).unwrap_or_else(|err| panic!("{name}: {err}"))
}

#[test]
fn fixtures_match_goldens_and_goldens_are_fixed_points() {
    for (name, source, expected) in fixtures() {
        assert_eq!(
            fmt(&name, &source),
            expected,
            "{name} drifted from its golden"
        );
        assert_eq!(
            fmt(&name, &expected),
            expected,
            "{name} golden is not canonical"
        );
    }
}

/// True when some line lies inside a multi-line block comment, whose
/// interior whitespace is payload rather than indentation.
fn has_multiline_block_comment(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.matches("/*").count() > line.matches("*/").count())
}

#[test]
fn layout_noise_does_not_change_canonical_output() {
    for (name, source, expected) in fixtures() {
        if !has_multiline_block_comment(&source) {
            let flat: String = source
                .lines()
                .map(|l| format!("{}\n", l.trim_start()))
                .collect();
            assert_eq!(fmt(&name, &flat), expected, "{name}: re-indent");
            let tabs: String = source
                .lines()
                .map(|l| {
                    let body = l.trim_start_matches(' ');
                    let depth = (l.len() - body.len()).div_ceil(2);
                    format!("{}{}\n", "\t".repeat(depth), body)
                })
                .collect();
            assert_eq!(fmt(&name, &tabs), expected, "{name}: tab indentation");
        }
        let crlf = source.replace('\n', "\r\n");
        assert_eq!(fmt(&name, &crlf), expected, "{name}: CRLF");
        let trailing: String = source
            .lines()
            .map(|l| {
                if l.contains("//") {
                    format!("{l}\n") // trailing space inside a comment is payload
                } else {
                    format!("{l}  \t\n")
                }
            })
            .collect();
        assert_eq!(
            fmt(&name, &trailing),
            expected,
            "{name}: trailing whitespace"
        );
    }
}

#[test]
fn every_prefix_formats_or_rejects_without_panicking() {
    for (_, source, _) in fixtures() {
        for (at, _) in source.char_indices() {
            let _ = format_source(&source[..at]);
        }
    }
}

// Keywords the formatter may insert, remove, reorder or rename. `val`, `let`,
// `const`, `final` and `mut` are deliberately absent: they must never change.
const REWRITABLE: &[&str] = &[
    "fnc",
    "routine",
    "do",
    "then",
    "elif",
    "elseif",
    "else",
    "if",
    "case",
    "when",
    "define",
    "as",
    "generator",
    "gen",
    "rt",
    "yield",
    "cooperate",
    "suspend",
    "suspends",
    "default",
    "on",
    "_",
    "pub",
    "private",
    "static",
    "async",
    "abstract",
    "shared",
    "untrusted",
    "actor",
    "isoactor",
    "class",
    "interface",
    "contract",
    "trait",
    "module",
    "struct",
    "quantum",
    "nlex",
    "pure",
    "trap",
    "structural",
    "forever",
    "loop",
    "while",
];

#[derive(Debug, PartialEq)]
enum Payload {
    Comment(String),
    Str(String),
    Num(String),
}

fn lex(source: &str) -> (Vec<Payload>, BTreeMap<String, usize>) {
    let chars: Vec<char> = source.chars().collect();
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    let (mut payload, mut idents) = (Vec::new(), BTreeMap::new());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('*') {
            let (mut depth, mut j) = (1, i + 2);
            while j < chars.len() && depth > 0 {
                match (chars[j], chars.get(j + 1).copied()) {
                    ('/', Some('*')) => {
                        depth += 1;
                        j += 2;
                    }
                    ('*', Some('/')) => {
                        depth -= 1;
                        j += 2;
                    }
                    _ => j += 1,
                }
            }
            payload.push(Payload::Comment(text(i, j).replace("\r\n", "\n")));
            i = j;
        } else if c == '/' && next == Some('/') {
            let j = (i..chars.len())
                .find(|&k| chars[k] == '\n')
                .unwrap_or(chars.len());
            payload.push(Payload::Comment(text(i, j).trim_end().to_string()));
            i = j;
        } else if matches!(c, '"' | '\'' | '`') {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != c {
                j += if chars[j] == '\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(chars.len());
            payload.push(Payload::Str(text(i, end)));
            i = end;
        } else if c.is_alphabetic() || c == '_' {
            let j = (i..chars.len())
                .find(|&k| !(chars[k].is_alphanumeric() || chars[k] == '_'))
                .unwrap_or(chars.len());
            let word = text(i, j);
            if !REWRITABLE.contains(&word.as_str()) {
                *idents.entry(word).or_insert(0) += 1;
            }
            i = j;
        } else if c.is_ascii_digit() {
            let j = (i..chars.len())
                .find(|&k| {
                    !(chars[k].is_ascii_alphanumeric() || chars[k] == '_' || chars[k] == '.')
                })
                .unwrap_or(chars.len());
            payload.push(Payload::Num(text(i, j)));
            i = j;
        } else {
            i += 1;
        }
    }
    (payload, idents)
}

fn assert_tokens_preserved(name: &str, source: &str, formatted: &str) {
    let (before_payload, before_idents) = lex(source);
    let (after_payload, after_idents) = lex(formatted);
    assert_eq!(
        before_payload, after_payload,
        "{name}: comment/string/number payload changed"
    );
    assert_eq!(
        before_idents, after_idents,
        "{name}: identifiers dropped or invented"
    );
}

#[test]
fn formatting_preserves_payload_and_identifiers() {
    for (name, source, expected) in fixtures() {
        assert_tokens_preserved(&name, &source, &expected);
    }
}

#[test]
fn round_trip_fixtures_preserve_payload_and_identifiers() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut checked = 0;
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let ext = path.extension().and_then(|e| e.to_str());
        if !matches!(ext, Some("atto" | "ores")) {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        let name = path.display().to_string();
        // Known negative: the legacy `.ores` copy uses the retired
        // callable-body `as`, which must stay rejected rather than skipped.
        if path
            .file_name()
            .is_some_and(|f| f == "end-delimited-roundtrip.ores")
        {
            let err = format_source(&source).expect_err("legacy `as` body must be rejected");
            assert!(
                err.message().contains("cannot use 'as'"),
                "{}",
                err.message()
            );
            continue;
        }
        assert_tokens_preserved(&name, &source, &fmt(&name, &source));
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn the_token_check_catches_a_dropped_val() {
    // Self-test: the invariant must detect the original `val` elision bug.
    let before = "define class A as\n  pub val int a = 1;\nend\n";
    let after = "define class A as\n  pub int a: 1;\nend\n";
    let result = std::panic::catch_unwind(|| assert_tokens_preserved("self-test", before, after));
    assert!(
        result.is_err(),
        "dropping `val` must violate token preservation"
    );
}
