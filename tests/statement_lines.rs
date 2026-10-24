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

#[test]
fn inline_braced_if_else_has_separate_control_and_action_lines() {
    let input = "fnc run() -> void {\nif ready { first(); second(); } else { fallback(); } fi\n}\n";
    let output = format_source(input).unwrap();
    assert!(output.contains("  if ready {\n"));
    assert!(output.contains("    first();\n"));
    assert!(output.contains("    second();\n"));
    assert!(output.contains("  else {\n"));
    assert!(output.contains("    fallback();\n"));
    assert!(!output.contains("first(); second();"));
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn formatter_does_not_invent_an_action_when_expanding_control_flow() {
    let formatted = format_source("fnc run() -> void {\nif foo then x(); fi\n}\n").unwrap();
    assert!(formatted.contains("    x();\n"));
    assert!(!formatted.contains("do();"));
}

#[test]
fn compact_keyword_if_nested_in_inline_brace_is_expanded() {
    let source = "fnc run() -> void {\nif outer { if inner then act(); fi }\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("  if outer {\n"));
    assert!(formatted.contains("    if inner then\n"));
    assert!(formatted.contains("      act();\n"));
    assert!(formatted.contains("    fi\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
