use atto_format::format_source;

#[test]
fn four_lambda_void_styles_have_identical_parameter_order_and_stable_layout() {
    let bodies = [
        "|int foo, string x| -> { return; };",
        "|foo: int, x: string| -> { return; }",
        "|foo: int, x: string| -> void { return; }",
        "|foo: int, x: string| : void -> { return; }",
    ];
    for body in bodies {
        let source = format!("fnc demo() -> void {{\\nconst v = {body}\\nreturn;\\n}}\\n");
        let once = format_source(&source).expect("four valid lambda spellings must format");
        assert!(once.contains("|int foo, string x|"), "{body}: {once}");
        assert_eq!(format_source(&once).unwrap(), once, "{body}");
        // Formatting must not turn a lambda's executable body arrow into
        // the => token reserved for callable type signatures.
        assert!(!once.contains("=> {"), "{once}");
    }
}

#[test]
fn colon_return_suffix_must_not_rewrite_string_and_comment_payloads() {
    let source = r#"fnc demo() -> void {
const s = \"|foo: int, x: string| : void -> {\";
// |foo: int, x: string| : void -> { comment payload }
const v = |foo: int, x: string| : void -> { return; };
return;
}
"#;
    let once = format_source(source).unwrap();
    assert!(once.contains("\\\"|foo: int, x: string| : void -> {\\\""), "{once}");
    assert!(once.contains("// |foo: int, x: string| : void ->"), "{once}");
    assert!(once.contains("|int foo, string x| : void ->"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}
