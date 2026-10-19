use oreslang_format::{format_source, is_formatted};

/// A structural record's source-origin spelling is observable to the
/// compiler AST. Formatting must not silently turn it into a struct or a map.
#[test]
fn inferred_record_and_struct_sources_keep_their_distinct_spellings() {
    let source = r#"fnc read() -> string {
const Person a = infer record{name: "Ada", age: 37};
const Person b = infer struct{age: 42, name: "Grace"};
const Contact c = infer<Contact> record{"first name": "Linus", "email@work": "a@b"};
const Metadata m = infer<Metadata> struct{value: "ok"};
return c["first name"];
}
"#;
    let output = format_source(source).expect("new closed-row expressions must format");
    for marker in [
        "infer record{",
        "infer struct{",
        "infer<Contact> record{",
        "infer<Metadata> struct{",
        r#""first name": "Linus""#,
        r#""email@work": "a@b""#,
        r#"c["first name"]"#,
    ] {
        assert!(output.contains(marker), "lost source token {marker:?}:\n{output}");
    }
    assert_eq!(format_source(&output).unwrap(), output, "formatter fixed point");
    assert!(is_formatted(&output).unwrap());
}

#[test]
fn closed_record_keys_with_braces_and_spaces_are_not_reinterpreted() {
    let source = r#"fnc read() -> string {
const T value = infer<T> record{
"}": "brace",
"  ": "spaces",
"emoji🚀": "rocket"
};
return value["}"];
}
"#;
    let output = format_source(source).unwrap();
    for literal in [
        r#""}": "brace""#,
        r#""  ": "spaces""#,
        r#""emoji🚀": "rocket""#,
        r#"value["}"]"#,
    ] {
        assert!(output.contains(literal), "quoted record field changed {literal:?}:\n{output}");
    }
    assert_eq!(format_source(&output).unwrap(), output);
}
