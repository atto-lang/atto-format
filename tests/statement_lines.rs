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

#[test]
fn multiline_braced_control_actions_have_one_statement_per_line() {
    let source = "fnc run() -> void {\nif ready { first(); second();\nthird(); fourth();\n} else { fallback(); }\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("  if ready {\n    first();\n    second();\n"));
    assert!(formatted.contains("    third();\n    fourth();\n"));
    assert!(formatted.contains("  }\n  else {\n    fallback();\n  }"));
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn multiline_data_literals_and_call_arguments_are_not_split() {
    let source = "fnc run() -> void {\nconst record = infer struct {\na: 1; b: 2\n}; next();\ncall(\n  first(); second(),\n  third()\n); finish();\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("a: 1; b: 2"));
    assert!(formatted.contains("first(); second(),"));
    assert!(formatted.contains("  next();"));
    assert!(formatted.contains("  finish();"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn inline_control_comment_and_string_payloads_are_stable() {
    let source = "fnc run() -> void {\nif ready { log(\"} else {;\"); /* if else; */ first(); second();\n} else { fallback(); }\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("log(\"} else {;\");"));
    assert!(formatted.contains("/* if else; */"));
    assert!(formatted.contains("  else {\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn multiline_callable_first_line_actions_are_separated() {
    let source = "fnc run() -> void { first(); second();\nthird(); fourth();\n}\n";
    let expected = "fnc run() -> void {\n  first();\n  second();\n  third();\n  fourth();\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn multiline_callable_preserves_comment_payload_and_statement_order() {
    let source = "fnc run() -> void { /* ; } */ first(); second();\nthird();\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("/* ; } */ first();\n"));
    assert!(formatted.contains("    second();\n") || formatted.contains("  second();\n"));
    assert!(formatted.contains("  third();\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn multiline_callable_line_comment_remains_on_opening_line() {
    let source = "fnc run() -> void { // body note; }\nfirst(); second();\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("{ // body note; }\n"));
    assert!(formatted.contains("  first();\n  second();\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn record_return_braces_are_not_treated_as_callable_bodies() {
    let source = "fnc build() -> { count: int } { first(); second();\nreturn value;\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("-> { count: int } {\n"));
    assert!(formatted.contains("  first();\n  second();\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn multiline_control_preserves_opening_line_comment_and_actions() {
    let source = "fnc run() -> void {\nif ready { // note: }; else; must remain comment\nfirst(); second();\n}\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("  if ready { // note: }; else; must remain comment\n"));
    assert!(formatted.contains("    first();\n    second();\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn control_header_literals_do_not_trigger_record_brace_guard() {
    let source = "fnc run() -> void {\nif name == \"infer struct\" { first(); second(); }\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("  if name == \"infer struct\" {\n"));
    assert!(formatted.contains("    first();\n    second();\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn block_comment_between_executable_actions_is_not_a_statement_shield() {
    let source = "fnc run() -> void {\nfirst(); /* an inline comment */ second();\nthird(); /* trailing only */\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("  first();\n  /* an inline comment */ second();\n"), "{formatted}");
    assert!(formatted.contains("  third(); /* trailing only */\n"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
