use oreslang_format::format_source;

// Proposed postfix exceptional-effect syntax must roundtrip without inventing,
// removing, or conflating throw/raise/panic contracts. This is a formatter
// preservation gate, NOT a claim that a compiler executes the syntax yet.
#[test]
fn preserves_optional_effect_annotations_for_fncs_and_methods() {
    let source = r#"pub fnc fetch(id: int) -> User throws(NetworkError, DecodeError) raises(AccessError) panics(InvariantError) {
  throw new NetworkError();
}

define class Worker as {
  method handle() -> void raises(Error) {
    raise new Error();
  }
  static fnc shutdown() -> void panics(Error) {
    panic new Error();
  }
}
"#;
    let formatted = format_source(source).expect("formatter must preserve proposed effect declarations");
    assert!(formatted.contains("throws(NetworkError, DecodeError)"));
    assert!(formatted.contains("raises(AccessError)"));
    assert!(formatted.contains("panics(InvariantError)"));
    assert!(formatted.contains("method handle() -> void raises(Error)"));
    assert!(formatted.contains("static fnc shutdown() -> void panics(Error)"));
    assert!(formatted.contains("raise new Error();"));
    assert!(formatted.contains("panic new Error();"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn keeps_exception_words_inside_literals_opaque() {
    let source = "fnc f() -> String {\n  return \"throws(Error) raises(Error) panics(Error)\";\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("\"throws(Error) raises(Error) panics(Error)\""));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
