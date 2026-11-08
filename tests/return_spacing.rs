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
