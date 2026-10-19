use oreslang_format::{format_source, is_formatted};

/// A structural record's source-origin spelling is observable to the
/// compiler AST. Formatting must not silently turn it into a struct or a map.
#[test]
fn inferred_record_and_struct_sources_keep_their_distinct_spellings() {
    let source = r#"fnc read() -> string {
const Row a = record{name: "Ada", age: 37};
const Point b = infer struct{age: 42, name: "Grace"};
const Contact c = infer<Contact> record{"first name": "Linus", "email@work": "a@b"};
const Metadata m = infer<Metadata> struct{value: "ok"};
return c["first name"];
}
"#;
    let output = format_source(source).expect("new closed-row expressions must format");
    assert!(output.contains("const Row a = record"), "canonical record gained infer intent:\n{output}");
    assert!(!output.contains("const Row a = infer record"), "formatter changed canonical record semantics:\n{output}");
    for marker in [
        "record",
        "infer struct",
        "infer<Contact> record",
        "infer<Metadata> struct",
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
const value = record{
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

#[test]
fn computed_keys_and_tuple_indices_preserve_index_expressions() {
    let source = r#"fnc access() -> string {
const Entry item = record{key: "id", value: "payload"};
const key = "value";
const T tupleValue = tuple(3.0, 4.0);
const index = 0;
const S objectValue = infer<S> struct{x: 3.0, y: 4.0};
const first = objectValue['x'];
const name = item[key];
const number = tupleValue[index];
return name;
}
"#;
    let formatted = format_source(source).expect("computed brackets format");
    for token in [
        "record",
        "tuple(",
        "infer<S> struct",
        "item[key]",
        "tupleValue[index]",
        "objectValue['x']",
    ] {
        assert!(formatted.contains(token), "lost token {token:?}:\\n{formatted}");
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn computed_keys_and_tuple_indices_preserve_index_expressions() {
    let source = r#"fnc access() -> string {
const Entry item = record{key: "id", value: "payload"};
const key = "value";
const T t = tuple(3.0, 4.0);
const index = 0;
const S d = infer<S> struct{x: 3.0, y: 4.0};
const first = d['x'];
const name = item[key];
const number = t[index];
return name;
}
"#;
    let formatted = format_source(source).expect("computed brackets format");
    for token in [
        "record",
        "tuple(",
        "infer<S> struct",
        "item[key]",
        "t[index]",
        "d['x']",
    ] {
        assert!(formatted.contains(token), "lost computed index token");
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}


#[test]
fn computed_fixed_product_access_preserves_dynamic_index_syntax() {
    let source = r#"fnc read(string key, int index) -> void {
const row = record{x: 3.0, y: "boo"};
const y = row[key];
const point = infer struct{x: 1.0, y: 2.0};
const x = point[key];
const pair = tuple(3.0, 4.0);
const slot = pair[index];
return;
}
"#;
    let output = format_source(source).expect("computed fixed-product access must format");
    for marker in ["row[key]", "point[key]", "pair[index]"] {
        assert!(output.contains(marker), "lost computed access {marker:?}:\n{output}");
    }
    assert_eq!(format_source(&output).unwrap(), output);
    assert!(is_formatted(&output).unwrap());
}
