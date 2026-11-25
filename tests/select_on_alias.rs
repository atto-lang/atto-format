use oreslang_format::{format_source, is_formatted};

#[test]
fn select_on_alias_canonicalizes_to_when_and_is_idempotent() {
    let source = "select over\non readch inbox: int item -> {\nuse(item);\n}\ndefault: {\nfinish();\n}\nend\n";
    let expected = "select over\n  when readch inbox: int item -> {\n    use(item);\n  }\n  default: {\n    finish();\n  }\nend\n";
    let formatted = format_source(source).expect("on alias should format");
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn select_on_alias_in_braced_and_nested_arms() {
    let source = "select {\non timeout 5s {}\ncase readch inbox: int item -> { use(item); }\ndefault: {}\n}\n";
    let formatted = format_source(source).expect("mixed select arm aliases should format");
    assert!(formatted.contains("when timeout 5s"));
    assert!(formatted.contains("when readch inbox: int item"));
    assert!(!formatted.contains("on timeout"));
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn select_on_alias_does_not_rewrite_match_arms_or_literals() {
    let source = "match state over\non #ready -> { emit(\"on readch\"); }\nend\n";
    let formatted = format_source(source).expect("match remains canonical on");
    assert!(formatted.contains("on #ready"));
    assert!(formatted.contains("\"on readch\""));
    assert!(!formatted.contains("when #ready"));
}
