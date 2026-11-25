use oreslang_format::{format_source, is_formatted};

#[test]
fn semicolon_inside_guest_string_is_never_treated_as_second_action() {
    let input = "fnc demo() -> string {\nreturn \"one;two\";\n}\n";
    let once = format_source(input).unwrap();
    assert!(once.contains("\"one;two\""), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
    assert!(is_formatted(&once).unwrap());
}

#[test]
fn control_keyword_text_inside_quoted_guest_payload_is_opaque() {
    let input = "fnc demo() -> string {\nreturn \"if; then; fi; do; end\";\n}\n";
    let once = format_source(input).unwrap();
    assert!(once.contains("\"if; then; fi; do; end\""), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}
