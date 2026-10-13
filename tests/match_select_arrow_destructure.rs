use oreslang_format::{format_source, is_formatted};

#[test]
fn canonicalizes_case_to_when_without_eating_record_projection_arrow() {
    let source = r#"fnc handle(Channel<{x:int,y:int}> messages) -> void {
do select {
case readch messages: let {x,y} -> { consume(x,y); }
case readch messages: const [int id, string name] -> { consume(id,name); }
default: -> { fallback(); }
}
}
"#;
    let formatted = format_source(source).expect("valid select projections should format");
    assert!(formatted.contains("when readch messages: let {x,y} -> {"), "{formatted}");
    assert!(formatted.contains(
        "when readch messages: const [int id, string name] -> {"
    ), "{formatted}");
    assert!(formatted.contains("default: -> {"), "{formatted}");
    assert!(!formatted.contains("case readch"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn leaves_canonical_do_match_record_projection_and_wildcard_intact() {
    let source = r#"fnc read(Option<{v:int}> message) -> int {
do match message over
on Some as {v} -> { return v; }
on _ -> { return 0; }
end
}
"#;
    let formatted = format_source(source).expect("valid do-match should format");
    assert!(formatted.contains("on Some as {v} -> {"), "{formatted}");
    assert!(formatted.contains("on _ -> {"), "{formatted}");
    assert!(!formatted.contains("default ->"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn legacy_brace_only_select_arm_is_still_formatable() {
    let source = r#"fnc receive(Channel<int> messages) -> void {
do select {
case readch messages: val msg { consume(msg); }
}
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("when readch messages: val msg {"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn typed_select_record_pattern_with_name_keeps_arrow() {
    let source = r#"fnc receive(Channel<{foo:int}> input) -> void {
do select {
when readch input: {foo: int} record -> { consume(record.foo); }
}
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("when readch input: {foo: int} record -> {"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
