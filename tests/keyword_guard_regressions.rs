use oreslang_format::format_source;

#[test]
fn identifier_functions_named_fi_and_else_remain_calls() {
    let source = "fnc f() {\nif ready then fi(); else(); fi\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("fi();"));
    assert!(output.contains("else();"));
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn return_continuations_after_blank_lines_are_not_separated() {
    let source = "fnc f() {\nreturn obj\n\n  .method()\n}\n";
    let output = format_source(source).unwrap();
    assert!(!output.contains("return obj;"));
}
