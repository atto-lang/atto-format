//! Lexical regression coverage only: the compiler owns operator admission/typing.
//! No dynamic dispatch, expression evaluation, or runtime parser is introduced.
use oreslang_format::format_source;

#[test]
fn keeps_attolang_equality_operator_spelling_and_is_idempotent() {
    let cases = [("eq", "7 eq 7"), ("neq", "7 neq 8"), ("!eq", "7 !eq 8")];
    for (name, expr) in cases {
        let input = format!("fnc check() -> bool {{\n  return {expr};\n}}\n");
        let once = format_source(&input).expect(name);
        assert!(once.contains(&format!("return {expr};")), "{name}: {once}");
        let twice = format_source(&once).expect(name);
        assert_eq!(once, twice, "{name}: formatter is not idempotent");
    }
}

#[test]
fn does_not_conflate_equality_with_binding_or_touch_literals_and_comments() {
    let input = r#"fnc check() -> bool {
  val x = 7;
  // x eq 7; x neq 8; x !eq 8; x == 7; x != 7
  val name = "eq neq !eq == !=";
  return x neq 8;
}
"#;
    let output = format_source(input).unwrap();
    assert!(output.contains("return x neq 8;"), "{output}");
    assert!(
        output.contains("x eq 7; x neq 8; x !eq 8; x == 7; x != 7"),
        "{output}"
    );
    assert!(output.contains(r#""eq neq !eq == !=""#), "{output}");
    assert_eq!(output, format_source(&output).unwrap());
}

#[test]
fn does_not_rewrite_unsupported_operators_into_valid_attolang_operators() {
    for expr in [
        "7 == 7",
        "7 != 8",
        "7 =:= 7",
        "7 ! eq 8",
        "7 !eqvalue 8",
        "7 neqvalue 8",
    ] {
        let input = format!("fnc check() -> bool {{\n  return {expr};\n}}\n");
        match format_source(&input) {
            Ok(output) => {
                // Rejecting invalid syntax is also safe, but if this formatter
                // passes it through, it must not delete or rewrite a byte of
                // the unsupported operator expression.
                assert!(
                    output.contains(&format!("return {expr};")),
                    "{expr}: modified unsupported expression: {output}"
                );
                assert!(
                    !output.contains("return 7 eq 7;"),
                    "{expr}: rewritten to eq"
                );
                assert!(
                    !output.contains("return 7 neq 8;"),
                    "{expr}: rewritten to neq"
                );
                assert!(
                    !output.contains("return 7 !eq 8;"),
                    "{expr}: rewritten to !eq"
                );
            }
            Err(_) => {} // Fail-closed formatter rejection is permitted.
        }
    }
}

#[test]
fn operator_keywords_are_not_substrings_of_longer_identifiers() {
    let src = r#"fnc check() -> bool {
  val eq_value = 7;
  val unequal_value = 8;
  // !eq is a prospective atomic binary token, not unary ! followed by eq
  return eq_value neq unequal_value;
}
"#;
    let once = format_source(src).expect("ordinary identifier spellings");
    assert!(once.contains("eq_value"), "{once}");
    assert!(once.contains("unequal_value"), "{once}");
    assert!(
        once.contains("return eq_value neq unequal_value;"),
        "{once}"
    );
    assert_eq!(once, format_source(&once).unwrap());
}

#[test]
fn no_operator_rewrites_inside_multiline_comments_and_strings() {
    let src = r#"fnc check() -> bool {
  /*
  7 == 7; 7 != 8; 7 eq 7; 7 neq 8; 7 !eq 8;
  */
  val label = "7 eq 7; 7 neq 8; 7 !eq 8;";
  return 7 !eq 8;
}
"#;
    let once = format_source(src).expect("valid operator source");
    assert!(
        once.contains("7 == 7; 7 != 8; 7 eq 7; 7 neq 8; 7 !eq 8;"),
        "{once}"
    );
    assert!(once.contains("\"7 eq 7; 7 neq 8; 7 !eq 8;\""), "{once}");
    assert!(once.contains("return 7 !eq 8;"), "{once}");
    assert_eq!(once, format_source(&once).unwrap());
}
