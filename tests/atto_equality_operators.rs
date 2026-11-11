use oreslang_format::{format_source, is_formatted};

#[test]
fn atto_inequality_alias_is_canonical_neq_and_idempotent() {
    let input = r#"fnc compare(int a, int b): bool {
return a !eq b and b eq a;
}
"#;
    let actual = format_source(input).unwrap();
    assert!(actual.contains("return a neq b and b eq a;"), "{actual}");
    assert!(!actual.contains("a !eq b"), "{actual}");
    assert!(is_formatted(&actual).unwrap());
    assert_eq!(format_source(&actual).unwrap(), actual);
}

#[test]
fn neq_alias_never_rewrites_string_or_comment_content() {
    let input = r#"fnc choose(int a, int b): bool {
  // !eq must remain verbatim
  /* block !eq must remain */
  stdio.out.write("literal !eq");
  return a !eq b;
}
"#;
    let actual = format_source(input).unwrap();
    assert!(actual.contains("// !eq must remain verbatim"), "{actual}");
    assert!(actual.contains("/* block !eq must remain */"), "{actual}");
    assert!(actual.contains("\"literal !eq\""), "{actual}");
    assert!(actual.contains("return a neq b;"), "{actual}");
    assert_eq!(format_source(&actual).unwrap(), actual);
}

#[test]
fn adjacent_left_operand_is_separated_and_other_comparisons_not_reinterpreted() {
    let input = r#"fnc compare(int a, int b): bool {
return a!eq b;
}
"#;
    let actual = format_source(input).unwrap();
    assert!(actual.contains("return a neq b;"), "{actual}");
    assert_eq!(format_source(&actual).unwrap(), actual);
}
