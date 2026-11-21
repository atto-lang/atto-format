use oreslang_format::format_source;

#[test]
fn canonicalizes_legacy_exception_statements_and_is_idempotent() {
    let source = r#"fnc f() -> void throws(Error) raises(Error) panics(Error) {
  throw new Error();
  raise new Error();
  panic new Error();
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("throw! new Error();"));
    assert!(formatted.contains("raise! new Error();"));
    assert!(formatted.contains("panic! new Error();"));
    assert!(formatted.contains("throws(Error) raises(Error) panics(Error)"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn preserves_existing_bangs_and_literals_comments_and_effects() {
    let source = r#"fnc f() -> void throws(Error) raises(Error) panics(Error) {
  // throw new Error(); raise new Error(); panic new Error();
  const note = "throw new Error(); raise new Error(); panic new Error();";
  /* throw new Error();
     raise new Error();
     panic new Error(); */
  throw! new Error();
  raise! new Error();
  panic! new Error();
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("// throw new Error(); raise new Error(); panic new Error();"));
    assert!(formatted.contains("\"throw new Error(); raise new Error(); panic new Error();\""));
    assert!(formatted.contains("/* throw new Error();"));
    assert!(formatted.contains("raise new Error();\n     panic new Error(); */"));
    assert!(!formatted.contains("throw!!"));
    assert!(!formatted.contains("raise!!"));
    assert!(!formatted.contains("panic!!"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn preserves_call_signature_effects_and_header_without_inventing_exceptions() {
    let source = "fnc f() -> void throws(Error) raises(Error) panics(Error) {\n  return;\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("throws(Error) raises(Error) panics(Error)"));
    assert!(!formatted.contains("throw!"));
    assert!(!formatted.contains("raise!"));
    assert!(!formatted.contains("panic!"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
