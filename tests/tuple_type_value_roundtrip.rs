use oreslang_format::{format_source, is_formatted};

#[test]
fn canonical_tuple_returns_and_value_constructors_remain_distinct() {
    let source = r#"pub fnc foo(): Tuple[string, string] {
return ~tuple("hello", "world");
}
fnc nested(): Tuple[int, Tuple[string, string]] {
return ~tuple(42, ~tuple("left", "right"));
}
"#;
    let formatted = format_source(source).expect("valid tuple source should be formattable");
    for fragment in [
        "Tuple[string, string]",
        "Tuple[int, Tuple[string, string]]",
        "~tuple(\"hello\", \"world\")",
        "~tuple(42, ~tuple(\"left\", \"right\"))",
    ] {
        assert!(formatted.contains(fragment), "lost {fragment}: {formatted}");
    }
    assert!(!formatted.contains("~ tuple("), "split the value-constructor sigil: {formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn zero_and_singleton_tuples_preserve_type_and_value_arity() {
    let source = r#"fnc empty(): Tuple[] {
return ~tuple();
}
fnc singleton(): Tuple[int] {
return ~tuple(42);
}
"#;
    let formatted = format_source(source).expect("zero/singleton tuples should format");
    for fragment in ["Tuple[]", "Tuple[int]", "~tuple()", "~tuple(42)"] {
        assert!(formatted.contains(fragment), "lost {fragment}: {formatted}");
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn legacy_nominal_angle_tuple_is_not_rewritten_into_intrinsic_brackets() {
    let source = r#"define class Tuple<A, B> as
end
fnc old(Tuple<int, string> pair): Tuple<int, string> {
return pair;
}
fnc built_in(): Tuple[int, string] {
return ~tuple(42, "value");
}
"#;
    let formatted = format_source(source).expect("legacy type spellings should round trip");
    assert!(formatted.contains("Tuple<int, string>"), "erased nominal type syntax: {formatted}");
    assert!(formatted.contains("Tuple[int, string]"), "erased intrinsic type syntax: {formatted}");
    assert!(formatted.contains("~tuple(42, \"value\")"), "rewrote tuple value: {formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn quoted_type_and_constructor_text_is_untouched() {
    let source = r#"fnc example(): Tuple[string, string] {
return ~tuple("Tuple[int, int]", "~tuple(value)");
}
"#;
    let formatted = format_source(source).expect("quoted tuple spellings should format");
    assert!(formatted.contains("\"Tuple[int, int]\""), "{formatted}");
    assert!(formatted.contains("\"~tuple(value)\""), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn explicit_array_and_tuple_constructors_keep_their_family_tokens() {
    let source = r#"fnc array_old(): Array[int] {
return arr[40, 2];
}
fnc array_new(): Array[int] {
return ~array(40, 2);
}
fnc tuple_new(): Tuple[int, int] {
return ~tuple(40, 2);
}
"#;
    let formatted = format_source(source).expect("array/tuple literals should format");
    for fragment in ["arr[40, 2]", "~array(40, 2)", "~tuple(40, 2)"] {
        assert!(formatted.contains(fragment), "changed aggregate family: {formatted}");
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
