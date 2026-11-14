//! AttoLang comma expression bodies preserve lexical punctuation, unlike
//! ordinary brace statement bodies with mandatory explicit value returns.
use oreslang_format::format_source;

#[test]
fn fnc_and_routine_comma_bodies_preserve_implicit_result_and_idempotence() {
    let input = r#"pub fnc plus(int n) -> int :: int x = 5, x + n end
pub routine twice(int n) -> int ::
  int factor = 2,
  factor * n
end
"#;
    let formatted = format_source(input).expect("format comma function bodies");
    assert!(formatted.contains("pub fnc plus(int n) -> int ::"));
    assert!(formatted.contains("int x = 5, x + n"));
    assert!(formatted.contains("pub routine twice(int n) -> int ::"));
    assert!(formatted.contains("int factor = 2,"));
    assert!(formatted.contains("factor * n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn class_and_receiver_methods_keep_comma_body_punctuation() {
    let input = r#"define class Calculator as
  pub add(int n) -> int ::
    int x = 5, x + n
  end
  pub mtd via_receiver(mut self, int n) -> int ::
    int x = 6,
    x + n
  end
  pub static fnc independent(int n) -> int ::
    n + 7
  end
end
"#;
    let formatted = format_source(input).expect("format class comma bodies");
    assert!(formatted.contains("pub add(int n) -> int ::"));
    assert!(formatted.contains("pub mtd via_receiver(mut self, int n) -> int ::"));
    assert!(formatted.contains("pub static fnc independent(int n) -> int ::"));
    assert!(formatted.contains("int x = 5, x + n"));
    assert!(formatted.contains("int x = 6,"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn brace_statements_and_comma_expressions_remain_distinct() {
    let source = r#"fnc statement_body(int n): int { return n + 5; }
fnc expression_body(int n) -> int ::
  int x = 5, x + n
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("return n + 5;"));
    assert!(formatted.contains("int x = 5, x + n"));
    assert!(formatted.contains("-> int ::"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn typed_inline_and_inferred_comma_lambdas_preserve_result_delimiters() {
    let input = r#"fnc main(): void {
  val typed = || -> int => 5 + 3;
  val inferred = || -> 5 + 3;
  val typed_local = || -> int => val int x = 5, x + 3;
  val inferred_local = || -> val int x = 5, x + 3;
  val block = || -> void :: 5 |> foo |> |x, y| -> stdio.println(x + y) end;
  val pipe = || -> void => 5 |> foo |> |x, y| -> stdio.println(x + y);
  return;
}
"#;
    let formatted = format_source(input).expect("typed and inferred lambdas");
    for important in [
        "|| -> int => 5 + 3",
        "|| -> 5 + 3",
        "|| -> int => val int x = 5, x + 3",
        "|| -> val int x = 5, x + 3",
        "|| -> void ::",
        "5 |> foo |> |x, y| -> stdio.println(x + y)",
        "|| -> void => 5 |> foo",
    ] {
        assert!(formatted.contains(important), "lost: {important}; formatted: {formatted}");
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn typed_pipeline_lambda_and_return_type_keeps_arrow_direction() {
    let source = r#"fnc main(): int {
  return 5 |> |n| -> int => val int x = n + 1, x * 2 |> increment;
}
"#;
    let formatted = format_source(source).expect("typed inline pipeline stage");
    assert!(formatted.contains("|n| -> int =>"));
    assert!(formatted.contains("val int x = n + 1, x * 2"));
    assert!(formatted.contains("|> increment"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
