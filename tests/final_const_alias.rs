use oreslang_format::{format_source, is_formatted};

#[test]
fn rewrites_proven_literal_final_const_fields_only_in_classes() {
    let source = r#"define class Fields as
pub final const int number: 5
final const bool enabled: true;
private final const string label: "a:b // literal"
final const int compatible = 7;
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("pub final int number: 5"), "{formatted}");
    assert!(formatted.contains("final bool enabled: true;"), "{formatted}");
    assert!(formatted.contains("private final string label: \"a:b // literal\""), "{formatted}");
    assert!(formatted.contains("final int compatible = 7;"), "{formatted}");
    assert!(!formatted.contains("final const"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn preserves_uncertain_final_const_expressions_and_legacy_bindings() {
    let source = r#"const int global = 1;
define module M as
const int module_value = 2;
define class Foo as
final const int computed: calculate()
final const int arithmetic: 1 + 2
final const int negative: -5
final const int suffix: 5 + next()
final const int overflowing: 9223372036854775808
final const mut Array<int> unsafe: [1, 2]
const int explicit_constant = 7;
fnc method(): void {
  final const int local: 3;
}
end
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("final const int computed: calculate()"), "{formatted}");
    assert!(formatted.contains("final const int arithmetic: 1 + 2"), "{formatted}");
    assert!(formatted.contains("final const int negative: -5"), "{formatted}");
    assert!(formatted.contains("final const int suffix: 5 + next()"), "{formatted}");
    assert!(
        formatted.contains("final const int overflowing: 9223372036854775808"),
        "{formatted}"
    );
    assert!(formatted.contains("final const mut Array<int> unsafe:"), "{formatted}");
    assert!(formatted.contains("const int explicit_constant = 7;"), "{formatted}");
    assert!(formatted.contains("final const int local: 3;"), "{formatted}");
    assert!(formatted.contains("const int global = 1;"), "{formatted}");
    assert!(formatted.contains("const int module_value = 2;"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn does_not_touch_actor_fields_comments_strings_or_nonliteral_suffixes() {
    let source = r#"// final const int commented: 5
define actor Actor as
final const int should_remain: 5;
end
define class Foo as
final /* preserve */ const int internal: 3
final const int duplicate: 3; /*
 multiline const final int literal: 5
*/
final const int with_suffix: 5 + 2
final const int with_comment: 5; // final const int inside comment
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("// final const int commented: 5"), "{formatted}");
    assert!(formatted.contains("final const int should_remain: 5;"), "{formatted}");
    assert!(formatted.contains("final /* preserve */ const int internal: 3"), "{formatted}");
    assert!(formatted.contains("final const int with_suffix: 5 + 2"), "{formatted}");
    assert!(formatted.contains("final int with_comment: 5; // final const int inside comment"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn never_strips_const_from_string_beyond_compiler_budget() {
    // The Java constexpr evaluator rejects decoded strings longer than
    // 65,536 UTF-16 code units. Raw-byte limiting is conservative.
    let too_large = "x".repeat(65_537);
    let src = format!(
        r#"define class C as
final const string large: "{}"
end
"#,
        too_large
    );
    let out = format_source(&src).unwrap();
    assert!(
        out.contains("final const string large:"),
        "const stripped on oversized field"
    );
    assert_eq!(format_source(&out).unwrap(), out);

    let at_limit = "x".repeat(65_536);
    let src = format!(
        r#"define class C as
final const string allowed: "{}"
end
"#,
        at_limit
    );
    let out = format_source(&src).unwrap();
    assert!(
        out.contains("final string allowed:"),
        "safe literal not normalized"
    );
    assert!(!out.contains("final const string allowed:"));
}
