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
