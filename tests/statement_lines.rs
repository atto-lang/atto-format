use oreslang_format::{format_source, is_formatted};

#[test]
fn compact_keyword_control_is_expanded_without_changing_the_action() {
    let source = "fnc run() -> void {\nif foo then x(); fi\n}\n";
    let expected = "fnc run() -> void {\n  if foo then\n    x();\n  fi\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn consecutive_actions_have_distinct_physical_lines() {
    let source = r#"fnc run() -> void {
first(); second(); third();
const text = "do(); then; fi";
log(text); finish(); // trailing statement comment
}
"#;
    let expected = r#"fnc run() -> void {
  first();
  second();
  third();
  const text = "do(); then; fi";
  log(text);
  finish(); // trailing statement comment
}
"#;
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn semicolons_in_arguments_are_not_separate_statements() {
    let source = "fnc run() -> void {\ncall(\"a;b\", nested(1, 2)); next();\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("  call(\"a;b\", nested(1, 2));\n  next();"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
