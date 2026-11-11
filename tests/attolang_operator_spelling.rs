//! Lexical regression coverage only: the compiler owns operator admission/typing.
//! No dynamic dispatch, expression evaluation, or runtime parser is introduced.
use oreslang_format::format_source;

#[test]
fn keeps_attolang_equality_operator_spelling_and_is_idempotent() {
    let cases = [
        ("eq", "7 eq 7"),
        ("neq", "7 neq 8"),
        ("!eq", "7 !eq 8"),
    ];
    for (name, expr) in cases {
        let input = format!("fnc check() -> bool as\n  return {expr};\nend\n");
        let once = format_source(&input).expect(name);
        assert!(once.contains(&format!("return {expr};")), "{name}: {once}");
        let twice = format_source(&once).expect(name);
        assert_eq!(once, twice, "{name}: formatter is not idempotent");
    }
}

#[test]
fn does_not_conflate_equality_with_binding_or_touch_literals_and_comments() {
    let input = r#"fnc check() -> bool as
  val x = 7;
  // x eq 7; x neq 8; x !eq 8; x == 7; x != 7
  val name = "eq neq !eq == !=";
  return x neq 8;
end
"#;
    let output = format_source(input).unwrap();
    assert!(output.contains("return x neq 8;"), "{output}");
    assert!(output.contains("x eq 7; x neq 8; x !eq 8; x == 7; x != 7"), "{output}");
    assert!(output.contains(r#""eq neq !eq == !=""#), "{output}");
    assert_eq!(output, format_source(&output).unwrap());
}

#[test]
fn does_not_rewrite_unsupported_operators_into_valid_attolang_operators() {
    for expr in ["7 == 7", "7 != 8", "7 =:= 7"] {
        let input = format!("fnc check() -> bool as\n  return {expr};\nend\n");
        if let Ok(output) = format_source(&input) {
            assert!(!output.contains("return 7 eq 7;"), "{expr}: rewritten to eq");
            assert!(!output.contains("return 7 neq 8;"), "{expr}: rewritten to neq");
            assert!(!output.contains("return 7 !eq 8;"), "{expr}: rewritten to !eq");
        }
    }
}
