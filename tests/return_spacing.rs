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
    assert!(
        formatted.contains("  return_value   = 10;\n"),
        "{formatted}"
    );
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

    assert!(
        formatted.contains("\"return   inside  string\""),
        "{formatted}"
    );
    assert!(
        formatted.contains("// return   inside comment"),
        "{formatted}"
    );
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

    assert!(
        formatted.contains("return   do_not_change_this_comment"),
        "{formatted}"
    );
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
    let gaps = [
        " ",
        "  ",
        "   ",
        "\t",
        "\t\t",
        " \t ",
        "\t  \t ",
        "          ",
    ];
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
    assert!(
        formatted.contains("return   more preserved text"),
        "{formatted}"
    );
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

#[test]
fn deterministic_horizontal_gap_stress_matrix() {
    // No external fuzz dependency: this exercises 384 reproducible cases.
    let expressions = [
        "x",
        "-x",
        "x + 1",
        "f(x)",
        "items[3]",
        "\"return   payload\"",
    ];
    let mut state = 0x9e3779b97f4a7c15_u64;
    for width in 1..=64 {
        let mut gap = String::with_capacity(width);
        for _ in 0..width {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            gap.push(if state & 1 == 0 { ' ' } else { '\t' });
        }
        for expression in expressions {
            let source = format!("fnc demo() -> int {{\nreturn{gap}{expression};\n}}\n");
            let expected = format!("fnc demo() -> int {{\n  return {expression};\n}}\n");
            assert_stable_format(&source, &expected);
        }
    }
}

#[test]
fn deterministic_bare_return_gap_stress_matrix() {
    // A padded empty return cannot become the different token sequence "return ;".
    for width in 1..=96 {
        let gap: String = (0..width)
            .map(|index| if index % 3 == 0 { '\t' } else { ' ' })
            .collect();
        let source = format!("fnc demo() -> void {{\nreturn{gap};\n}}\n");
        assert_stable_format(&source, "fnc demo() -> void {\n  return;\n}\n");
    }
}

#[test]
fn return_spacing_is_consistent_with_optional_mtd_pass() {
    use oreslang_format::format_source_with_mtd;

    let source = "pub fnc demo() -> int {\nreturn\t   11;\n}\n";
    let expected = "pub fnc demo() -> int {\n  return 11;\n}\n";
    assert_eq!(format_source_with_mtd(source).unwrap(), expected);
    assert_eq!(format_source_with_mtd(expected).unwrap(), expected);
}

#[test]
fn negative_and_boolean_return_expressions_keep_their_operators() {
    let source = r#"fnc demo() -> int {
return    -7;
return	!ready;
return   x == y;
return   value + (offset * 2);
}
"#;
    let expected = r#"fnc demo() -> int {
  return -7;
  return !ready;
  return x == y;
  return value + (offset * 2);
}
"#;
    assert_stable_format(source, expected);
}

#[test]
fn empty_returns_preserve_following_comment_payload() {
    let source = "fnc demo() -> void {\nreturn   ; //  reason   kept  \n}\n";
    let expected = "fnc demo() -> void {\n  return; //  reason   kept  \n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn return_expression_can_start_with_block_comment() {
    let source = "fnc demo() -> int {\nreturn   /* explain   */ x;\n}\n";
    let expected = "fnc demo() -> int {\n  return /* explain   */ x;\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn return_expression_preserves_inner_block_comment_spacing() {
    let source = "fnc demo() -> int {\nreturn \t x /*  keep   text */ + 2;\n}\n";
    let expected = "fnc demo() -> int {\n  return x /*  keep   text */ + 2;\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn return_expression_preserves_escaped_backslashes_and_unicode() {
    let source = r#"fnc demo() -> string {
return    "path \\srv\\share return   🦀 東京";
}
"#;
    let expected = r#"fnc demo() -> string {
  return "path \\srv\\share return   🦀 東京";
}
"#;
    assert_stable_format(source, expected);
}

#[test]
fn non_return_names_do_not_match_keyword_prefix() {
    let identifiers = [
        "return_value",
        "returned",
        "returning",
        "returns",
        "return2",
    ];
    for identifier in identifiers {
        let source = format!("fnc demo() -> int {{\n{identifier}   = 42;\n}}\n");
        let expected = format!("fnc demo() -> int {{\n  {identifier}   = 42;\n}}\n");
        assert_stable_format(&source, &expected);
    }
}

#[test]
fn return_without_whitespace_separator_stays_unmodified() {
    for expression in ["(x)", "[0]", ".value"] {
        let source = format!("fnc demo() -> int {{\nreturn{expression};\n}}\n");
        let expected = format!("fnc demo() -> int {{\n  return{expression};\n}}\n");
        assert_stable_format(&source, &expected);
    }
}

#[test]
fn leading_indentation_does_not_change_statement_payload() {
    let source = "fnc demo() -> int {\n\t   return \t  foo( 1,  2 );\n}\n";
    let expected = "fnc demo() -> int {\n  return foo( 1,  2 );\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn return_keywords_inside_comments_remain_opaque() {
    let source = r#"fnc demo() -> int {
// return    111;
/* return     222; */
return   333;
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("// return    111;"), "{formatted}");
    assert!(formatted.contains("/* return     222; */"), "{formatted}");
    assert!(formatted.contains("  return 333;"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn broken_brace_input_is_not_mistaken_for_valid_return_source() {
    let invalid = "fnc demo() -> int {\nreturn   7;\n";
    assert!(format_source(invalid).is_err());
}

#[test]
fn unterminated_return_string_is_not_silently_rewritten() {
    let invalid = "fnc demo() -> string {\nreturn   \"unfinished\n}\n";
    assert!(format_source(invalid).is_err());
}

#[test]
fn repeated_idempotence_after_return_normalization() {
    let source = "fnc demo() -> int {\nreturn    f( x );\n}\n";
    let mut formatted = format_source(source).unwrap();
    for _ in 0..10 {
        let next = format_source(&formatted).unwrap();
        assert_eq!(next, formatted);
        formatted = next;
    }
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn comment_only_bare_return_preserves_comment_payload() {
    let source = "fnc demo() -> void {\nreturn    //  reason   kept\n}\n";
    let expected = "fnc demo() -> void {\n  return;    //  reason   kept\n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn multiline_return_continuation_is_not_terminated_early() {
    let source = "fnc demo() -> int {\nreturn     value\n  .transform();\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("return value\n"), "{formatted}");
    assert!(!formatted.contains("return value;\n"), "{formatted}");
    assert!(formatted.contains(".transform();"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn inline_keyword_return_preserves_control_flow_and_idempotence() {
    let source = "fnc demo() -> int {\nif ready then return     3; fi\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("return 3;"), "{formatted}");
    assert!(formatted.contains("fi"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn return_string_literals_with_fake_delimiters_remain_opaque() {
    let source = r#"fnc demo() -> string {
return     "} ; /* a // b */ { return  inner";
}
"#;
    let expected = r#"fnc demo() -> string {
  return "} ; /* a // b */ { return  inner";
}
"#;
    assert_stable_format(source, expected);
}

#[test]
fn long_horizontal_gap_is_bounded_by_input_size_and_idempotent() {
    let gap = " \t".repeat(4096);
    let source = format!("fnc demo() -> int {{\nreturn{gap}1;\n}}\n");
    assert_stable_format(&source, "fnc demo() -> int {\n  return 1;\n}\n");
}

#[test]
fn return_spacing_keeps_trailing_comment_with_unicode_and_tabs() {
    let source = "fnc demo() -> int {\nreturn \t  8; //  東京 \t 🦀 \t \n}\n";
    let expected = "fnc demo() -> int {\n  return 8; //  東京 \t 🦀 \t \n}\n";
    assert_stable_format(source, expected);
}

#[test]
fn large_mixed_body_only_rewrites_return_separator() {
    // The formatting pass must not accidentally rewrite unrelated statements.
    // This also exercises a body dominated by non-return lines.
    let mut source = String::from("fnc demo() -> int {\n");
    let mut expected = source.clone();
    for index in 0..256 {
        let line = format!("  val v{index} = {index};\n");
        source.push_str(&line);
        expected.push_str(&line);
    }
    source.push_str("  // return    in   comment\n  return  \t  7;\n}\n");
    expected.push_str("  // return    in   comment\n  return 7;\n}\n");
    assert_stable_format(&source, &expected);
}
