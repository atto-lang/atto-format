use oreslang_format::format_source;

#[test]
fn canonical_select_record_projection_keeps_slim_arrow_and_is_idempotent() {
    let src = r#"do select {
  when readch records: let {x,y} -> {
    use(x);
    use(y);
  }
  default: -> {
    fallback();
  }
}
"#;
    let once = format_source(src).expect("format canonical select");
    assert!(once.contains("when readch records: let {x,y} -> {"), "{once}");
    assert!(once.contains("default: -> {"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn typed_tuple_and_record_select_patterns_are_not_split_as_bodies() {
    let src = r#"do select {
  when readch tuples: const [_, string label] -> { use(label); }
  when readch records: const {foo: string} -> { use(foo); }
  when readch records: {foo} -> { use(foo); }
  when readch numbers: int number -> { use(number); }
}
"#;
    let formatted = format_source(src).expect("format pattern select");
    assert!(formatted.contains("const [_, string label] -> {"), "{formatted}");
    assert!(formatted.contains("const {foo: string} -> {"), "{formatted}");
    assert!(formatted.contains("readch records: {foo} -> {"), "{formatted}");
    assert!(formatted.contains("int number -> {"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn nested_record_type_in_select_header_keeps_single_arm_arrow() {
    let src = r#"do select {
  when readch data: const payload: {foo: {bar: int}} -> {
    use(payload);
  }
}
"#;
    let once = format_source(src).expect("nested record projection");
    assert!(once.contains("const payload: {foo: {bar: int}} -> {"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn select_legacy_braced_body_remains_distinct_from_record_pattern() {
    let src = r#"select {
  when readch input: {
    // A lambda's arrow is not the select arm arrow.
    use(|x| -> x);
  }
}
"#;
    let formatted = format_source(src).expect("format legacy arm");
    assert!(formatted.contains("when readch input: {"), "{formatted}");
    assert!(!formatted.contains("when readch input: -> {"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn malformed_explicit_arrow_without_block_fails_closed() {
    let source = "select { when readch input: int n -> print(n); }";
    let error = format_source(source).expect_err("missing braced arm body must fail");
    assert!(error.message().contains("slim arrow must be followed by a braced body"));
}

#[test]
fn do_match_as_record_projection_retains_arm_boundary() {
    let input = r#"fnc examine(Option<{v: int}> item): void {
  do match item over
    on Some as {v} -> {
      use(v);
    }
    on None -> {}
  end
}
"#;
    let once = format_source(input).expect("format do match");
    assert!(once.contains("on Some as {v} -> {"), "{once}");
    assert!(once.contains("on None -> {"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}
