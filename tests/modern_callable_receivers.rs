//! Merged Atto spec #19/#37 callable receiver and suspension layout.
//! Syntax formatting, not executable backend parity.
use oreslang_format::{format_source, is_formatted};

fn fixed_point(source: &str) -> String {
    let first = format_source(source).expect("format callable source");
    assert_eq!(first, format_source(&first).unwrap(), "unstable callable output");
    assert!(is_formatted(&first).unwrap());
    first
}

#[test]
fn out_of_line_receivers_canonicalize_only_equivalent_nominal_spelling() {
    let forms = [
        ("Calculator self", "self: Calculator"),
        ("Calculator _", "Calculator"),
        ("Calculator", "Calculator"),
        ("self: Calculator", "self: Calculator"),
        ("mut self: Calculator", "mut self: Calculator"),
    ];
    for (written, expected) in forms {
        let source = format!(
            "mtd ({written}) square(int x) -> int {{\nreturn x * x;\n}}\n"
        );
        let output = fixed_point(&source);
        assert!(
            output.contains(&format!("mtd ({expected}) square(int x) -> int {{")),
            "receiver form {written}: {output}"
        );
    }
}

#[test]
fn mtd_normalization_rewrites_method_arguments_not_receiver_binding() {
    let output = fixed_point(
        "mtd (self: Calculator) mix(a: int, int b): int {\nreturn a+b;\n}\n"
    );
    assert!(output.contains("mtd (self: Calculator) mix(int a, int b) -> int {"), "{output}");
    assert!(!output.contains("mtd (Calculator self)"), "{output}");
}

#[test]
fn mtd_compact_erlang_body_expands_without_semantics_changing_return() {
    let input = "mtd (Calculator self) square(int x) -> int :: x * x end\n";
    let output = fixed_point(input);
    assert!(output.contains("mtd (self: Calculator) square(int x) -> int ::\n"), "{output}");
    assert!(output.contains("  x * x\nend\n"), "{output}");
    assert!(!output.contains("return x * x"), "{output}");
    assert!(!output.contains("x * x;"), "{output}");
}

#[test]
fn mtd_multiline_comma_tail_preserves_erlang_sequence_not_semicolons() {
    let source = r#"mtd (mut self: Counter) increment(int amount) -> int ::
self.count = self.count + amount,
self.count
end
"#;
    let output = fixed_point(source);
    assert!(output.contains("mtd (mut self: Counter) increment(int amount) -> int ::"), "{output}");
    assert!(output.contains("self.count = self.count + amount,"), "{output}");
    assert!(!output.contains("amount;"), "{output}");
    assert!(output.contains("  self.count\nend"), "{output}");
}

#[test]
fn second_body_arrow_after_colon_type_remains_brace_style() {
    // Spec #19: ': Type -> {' is a C-style body, not two return arrows.
    let source = r#"fnc value(): int -> { return 42; }
mtd (self: Calculator) square(int x): int -> { return x * x; }
"#;
    let output = fixed_point(source);
    assert!(output.contains("fnc value():"), "{output}");
    assert!(output.contains("int -> {"), "{output}");
    assert!(output.contains("square(int x): int -> {"), "{output}");
    assert!(!output.contains("-> int -> {"), "{output}");
}

#[test]
fn direct_methods_keep_receiver_and_c_style_distinct_from_erlang_style() {
    let source = r#"define class Calculator as
fnc add(int a, int b): int { return a + b; }
divide(int a, int b) -> int ::
  let int result = a / b,
  result
end
end
"#;
    let output = fixed_point(source);
    assert!(output.contains("  add(int a, int b) -> int {"), "{output}");
    assert!(output.contains("  divide(int a, int b) -> int ::"), "{output}");
    assert!(output.contains("    let int result = a / b,"), "{output}");
    assert!(output.contains("    result\n  end"), "{output}");
}

#[test]
fn modern_susp_and_legacy_effect_spelling_are_preserved_not_laundered() {
    // Spec #37 canonically recommends susp, but Java/LLVM/.NET acceptance
    // must be separately verified before auto-migrating old source.
    for modifier in ["susp", "suspends", "suspend"] {
        let source = format!(
            "pub {modifier} fnc pending(): int {{\nreturn 42;\n}}\n"
        );
        let output = fixed_point(&source);
        assert!(
            output.contains(&format!("pub {modifier} fnc pending()")),
            "effect {modifier} was silently rewritten: {output}"
        );
    }
}

#[test]
fn quoted_receiver_like_text_and_contextual_mtd_calls_stay_opaque() {
    let source = r#"fnc example(): void {
  const string text = "mtd (Calculator self) square(int x)";
  mtd(value);
  return;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains(r#""mtd (Calculator self) square(int x)""#), "{output}");
    assert!(output.contains("  mtd(value);"), "{output}");
}

#[test]
fn duplicate_or_unknown_receiver_forms_are_not_repaired_by_guesswork() {
    for receiver in ["mut Calculator", "self: T: U", "Calculator self other"] {
        let source = format!(
            "mtd ({receiver}) square(int x) -> int {{\nreturn x;\n}}\n"
        );
        let output = fixed_point(&source);
        assert!(
            output.contains(&format!("mtd ({receiver})")),
            "unknown receiver source was reinterpreted: {output}"
        );
    }
}
