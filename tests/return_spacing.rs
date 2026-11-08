use oreslang_format::{format_source, is_formatted};

#[test]
fn collapses_extra_space_in_return_without_rewriting_other_statements() {
    let source = "pub fnc main() -> int {\n  val x = 10;\n  x = 20;\n  return   x;\n}\n";
    let expected = "pub fnc main() -> int {\n  val x = 10;\n  x = 20;\n  return x;\n}\n";

    assert_eq!(format_source(source).unwrap(), expected);
    assert_eq!(format_source(expected).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
    assert!(!is_formatted(source).unwrap());
}

#[test]
fn only_collapses_horizontal_whitespace_after_visible_return_keyword() {
    let source = "fnc demo() -> int {\nreturn\t  \tx;\nreturn_value   = 10;\nreturn;\n}\n";
    let formatted = format_source(source).unwrap();

    assert!(formatted.contains("  return x;\n"), "{formatted}");
    assert!(formatted.contains("  return_value   = 10;\n"), "{formatted}");
    assert!(formatted.contains("  return;\n"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn return_spacing_does_not_touch_expression_literals_or_comments() {
    let source = r#"fnc demo() -> string {
val s = "return   inside  string";
// return   inside comment
return   s; // leave   comment   payload
}
"#;
    let formatted = format_source(source).unwrap();

    assert!(formatted.contains("\"return   inside  string\""), "{formatted}");
    assert!(formatted.contains("// return   inside comment"), "{formatted}");
    assert!(
        formatted.contains("  return s; // leave   comment   payload"),
        "{formatted}"
    );
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn return_word_inside_multiline_comment_is_opaque() {
    let source = r#"fnc demo() -> int {
/*
return   do_not_change_this_comment
*/
return   1;
}
"#;
    let formatted = format_source(source).unwrap();

    assert!(formatted.contains("return   do_not_change_this_comment"), "{formatted}");
    assert!(formatted.contains("  return 1;"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

fn assert_stable_format(source: &str, expected: &str) {
    let once = format_source(source).unwrap_or_else(|error| panic!("{source:?}: {error}"));
    assert_eq!(once, expected, "source: {source:?}");
    assert_eq!(format_source(&once).unwrap(), once);
    assert!(is_formatted(&once).unwrap());
}

#[test]
fn table_driven_return_gaps_and_expression_forms() {
    let gaps = [" ", "  ", "   ", "\t", "\t\t", " \t ", "\t  \t ", "          "];
    let expressions = [
        "0",
        "value",
        "value + 2",
        "(value)",
        "compute(value)",
        "items[0]",
        "\"quoted  value\"",
        "\"mañana 東京\"",
    ];

    for gap in gaps {
        for expression in expressions {
            let source = format!("fnc demo() -> int {{\nreturn{gap}{expression};\n}}\n");
            let expected = format!("fnc demo() -> int {{\n  return {expression};\n}}\n");
            assert_stable_format(&source, &expected);
        }
    }
}

#[test]
fn bare_return_has_no_space_before_semicolon() {
    for gap in ["", " ", "  ", "\t", " \t ", "\t\t  "] {
        let source = format!("fnc demo() -> void {{\nreturn{gap};\n}}\n");
        assert_stable_format(&source, "fnc demo() -> void {\n  return;\n}\n");
    }
}

#[test]
fn preserves_whitespace_within_return_expression() {
    let source = "fnc demo() -> int {\nreturn   compute(  a,   b ) +  other;\n}\n";
    let expected = "fnc demo() -> int {\n  return compute(  a,   b ) +  other;\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn preserves_trailing_line_comment_payload() {
    let source = "fnc demo() -> int {\nreturn \t  7; //  explain   why  \n}\n";
    let expected = "fnc demo() -> int {\n  return 7; //  explain   why  \n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn preserves_block_comment_within_return_expression() {
    let source = "fnc demo() -> int {\nreturn   value /*  note   here  */ + 1;\n}\n";
    let expected = "fnc demo() -> int {\n  return value /*  note   here  */ + 1;\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn keeps_escaped_quote_payload_untouched() {
    let source = r#"fnc demo() -> string {
return   "say \"return   inside\"   again";
}
"#;
    let expected = r#"fnc demo() -> string {
  return "say \"return   inside\"   again";
}
"#;
    assert_stable_format(source, expected);
}

#[test]
fn line_comment_containing_return_is_not_executable_code() {
    let source = "fnc demo() -> int {\n// return   is documentation\nreturn   5;\n}\n";
    let expected = "fnc demo() -> int {\n  // return   is documentation\n  return 5;\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn return_prefixed_identifiers_are_not_return_statements() {
    let source = r#"fnc demo() -> int {
return_value   = 1;
returning   = 2;
returned   = 3;
return   returning;
}
"#;
    let expected = r#"fnc demo() -> int {
  return_value   = 1;
  returning   = 2;
  returned   = 3;
  return returning;
}
"#;
    assert_stable_format(source, expected);
}

#[test]
fn multiple_return_branches_obey_same_rule() {
    let source = r#"fnc demo() -> int {
if ready then
return    1;
else
return	 2;
fi
}
"#;
    let expected = r#"fnc demo() -> int {
  if ready then
    return 1;
  else
    return 2;
  fi
}
"#;
    assert_stable_format(source, expected);
}

#[test]
fn compact_callable_body_normalizes_return_spacing() {
    let source = "pub fnc demo() -> int { return   12; }\n";
    let expected = "pub fnc demo() -> int {\n  return 12;\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn crlf_and_mixed_indent_normalize_with_return_spacing() {
    let source = "fnc demo() -> int {\r\n\treturn\t   value;   \r\n}\r\n";
    let expected = "fnc demo() -> int {\n  return value;\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn block_comment_with_return_keyword_stays_opaque() {
    let source = r#"fnc demo() -> int {
/*
return   preserved text
return   more preserved text
*/
return     6;
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("return   preserved text"), "{formatted}");
    assert!(formatted.contains("return   more preserved text"), "{formatted}");
    assert!(formatted.contains("  return 6;"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn return_expression_with_no_separator_does_not_gain_one() {
    let source = "fnc demo() -> int {\nreturn(value);\n}\n";
    let expected = "fnc demo() -> int {\n  return(value);\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn line_with_return_in_string_does_not_rewrite_the_string() {
    let source = r#"fnc demo() -> string {
val message = "return    x; // keep";
return     message;
}
"#;
    let expected = r#"fnc demo() -> string {
  val message = "return    x; // keep";
  return message;
}
"#;
    assert_stable_format(source, expected);
}

#[test]
fn already_canonical_return_is_a_fixed_point() {
    let source = "fnc demo() -> int {\n  return foo(1, 2);\n}\n";
    assert_stable_format(source, source);
}
