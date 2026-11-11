use oreslang_format::{format_source, is_formatted};

// The formatter must never insert a semicolon after a ternary arm's implicit
// result expression; ordinary standalone brace blocks do not produce values.
#[test]
fn preserves_ternary_block_values_and_range_spelling() {
    let source = r#"pub routine main(): void {
  for id in 1..1024 {
    val next = id eq 1023 ? {
      val answer = 42;
      answer
    } : {
      id + 1
    };
    stdio.stdout.write(next);
  }
  for id in 1..=80 {
    stdio.stdout.write(id);
  }
}
"#;

    let once = format_source(source).unwrap();
    assert!(once.contains("for id in 1..1024"), "{once}");
    assert!(once.contains("for id in 1..=80"), "{once}");
    assert!(once.contains("id eq 1023 ? {"), "{once}");
    assert!(once.contains("} : {"), "{once}");
    assert!(
        once.contains("answer\n"),
        "value-block tail lost or terminated: {once}"
    );
    assert!(
        !once.contains("answer;\n"),
        "value-block tail was terminated: {once}"
    );
    assert_eq!(format_source(&once).unwrap(), once);
    assert!(is_formatted(&once).unwrap());
}

#[test]
fn preserves_ternary_arm_tail_and_literal_comments() {
    let source = r#"pub routine main(): void {
  val result: int = true ? {
    val left = 20;
    val right = 22;
    left + right
  } : {
    return 0;
  };
  // A comment with ? { } : { } and 1..=80 is not executable source.
  stdio.stdout.write("literal ? { } : { } 1..1024");
  stdio.stdout.write(result);
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("left + right\n"), "{formatted}");
    assert!(!formatted.contains("left + right;"), "{formatted}");
    assert!(
        formatted.contains("// A comment with ? { } : { } and 1..=80"),
        "{formatted}"
    );
    assert!(
        formatted.contains("\"literal ? { } : { } 1..1024\""),
        "{formatted}"
    );
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn keeps_ternary_thunks_distinct_from_immediate_typed_lambda_calls() {
    let source = r#"pub routine main(): void {
  for id in 1..1024 {
    val thunked: Array<int> = id eq 1023 ? || -> {
      return []
    } : || -> {
      return [id + 1]
    };
    val called: Array<int> = id eq 1023 ? (|int v| -> {
      return []
    })(3) : (|int g| -> {
      return [id + 1]
    })(5);
  }
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("? || -> {"), "{formatted}");
    assert!(formatted.contains("} : || -> {"), "{formatted}");
    assert!(formatted.contains("(|int v| -> {"), "{formatted}");
    assert!(formatted.contains("(|int g| -> {"), "{formatted}");
    assert!(formatted.contains("})(3)"), "{formatted}");
    assert!(formatted.contains("})(5)"), "{formatted}");
    assert!(formatted.contains("return []"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
