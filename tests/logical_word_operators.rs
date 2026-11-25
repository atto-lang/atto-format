use oreslang_format::{format_source, is_formatted};

#[test]
fn word_logical_operators_keep_multiline_returns_open() {
    let input = r#"fnc choose(bool a, bool b, bool c): bool {
return a and
b or
c
}
"#;
    let formatted = format_source(input).unwrap();
    assert!(!formatted.contains("return a and;"));
    assert!(!formatted.contains("b or;"));
    assert!(formatted.contains("return a and"));
    assert!(formatted.contains("b or"));
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn leading_word_operator_continuations_are_not_split_by_semicolons() {
    let input = r#"fnc choose(bool a, bool b): bool {
return a
or b
}
"#;
    let formatted = format_source(input).unwrap();
    assert!(!formatted.contains("return a;"));
    assert!(formatted.contains("or b"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn symbolic_logical_spelling_remains_unchanged() {
    let input = r#"fnc choose(bool a, bool b): bool {
return a && b || a;
}
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("a && b || a"));
}
