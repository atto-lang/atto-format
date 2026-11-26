use oreslang_format::{format_source, is_formatted};

#[test]
fn canonicalizes_spaced_and_legacy_ownership_prefixes() {
    for (input, expected) in [
        ("copy!   foo", "copy! foo"),
        ("rt copy foo", "copy! foo"),
        ("rt take foo", "take! foo"),
        ("rt borrow foo", "borrow! foo"),
        ("borrow! mut foo", "borrow! mut foo"),
        ("borrow! mutfoo", "borrow! mutfoo"),
        // The complete 'mut' keyword is required; mutfoo stays the name of
        // an immutable-loan operand even in the compact source form.
        ("borrow! mut(value)", "borrow! mut (value)"),
        ("mut borrow! foo", "borrow! mut foo"),
        ("rt mut borrow foo", "borrow! mut foo"),
        ("rt borrow mut foo", "borrow! mut foo"),
        ("rt mut borrow(foo)", "borrow! mut (foo)"),
        ("rt share foo", "share! foo"),
        ("rt copy !foo", "copy! !foo"),
    ] {
        let source = format!("fnc f(): int {{\n  return {input};\n}}\n");
        let output = format_source(&source).expect(input);
        assert!(output.contains(&format!("return {expected};")), "{input}: {output}");
        assert_eq!(format_source(&output).unwrap(), output, "idempotence {input}");
        assert!(is_formatted(&output).unwrap(), "canonical {input}");
    }
}

#[test]
fn does_not_turn_ordinary_comparisons_or_negation_into_ownership_ops() {
    let source = "fnc f(): bool {\nreturn copy!=foo;\nreturn copy!eq;\nreturn !foo;\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("copy!=foo;"), "{output}");
    assert!(output.contains("copy!eq;"), "{output}");
    let neq = format_source("fnc f(): bool {\n  return copy!eq foo;\n}\n").unwrap();
    assert!(neq.contains("copy!eq foo;"), "{neq}");
    assert!(!neq.contains("copy! eq foo;"), "{neq}");
    let unicode_neq = format_source("fnc f(): bool {\n  return copy!eq é;\n}\n").unwrap();
    assert!(unicode_neq.contains("copy!eq é;"), "{unicode_neq}");
    assert!(!unicode_neq.contains("copy! eq é;"), "{unicode_neq}");
    assert!(output.contains("return !foo;"), "{output}");
    let mutable_comparison = format_source(
        "fnc f(): bool {\n  return mut borrow!eq foo;\n}\n").unwrap();
    assert!(mutable_comparison.contains("mut borrow!eq foo;"),
            "{mutable_comparison}");
    assert!(!mutable_comparison.contains("borrow! mut eq foo;"),
            "{mutable_comparison}");
}

#[test]
fn never_rewrites_literals_or_comments_or_ownership_like_identifiers() {
    let source = "fnc f(): void {\n  stdio.println(\"rt take foo, copy!foo, mut borrow!foo\");\n  // rt borrow mut foo, mut borrow!bar, share!bar\n  val copied = 1;\n  val view = rt /* preserved */ copy foo;\n  return;\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("\"rt take foo, copy!foo, mut borrow!foo\""));
    assert!(output.contains("// rt borrow mut foo, mut borrow!bar, share!bar"));
    assert!(output.contains("val copied = 1;"));
    assert!(output.contains("rt /* preserved */ copy foo;"));
}

#[test]
fn mixed_legacy_and_bang_tokens_never_create_double_bang() {
    let source = "fnc f(): int {\n  return rt copy! foo;\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(!formatted.contains("copy!!"), "{formatted}");
    assert!(formatted.contains("rt copy! foo;"), "{formatted}");
}

#[test]
fn dense_prefixes_format_linearly_without_losing_tokens() {
    let nested = "copy! ".repeat(256) + "foo";
    let source = format!("fnc f(): int {{\n  return {nested};\n}}\n");
    let result = format_source(&source).unwrap();
    assert_eq!(result.matches("copy! ").count(), 256);
    assert_eq!(format_source(&result).unwrap(), result);

    let legacy = "rt copy ".repeat(256) + "foo";
    let source = format!("fnc f(): int {{\n  return {legacy};\n}}\n");
    let result = format_source(&source).unwrap();
    assert_eq!(result.matches("copy! ").count(), 256);
    assert_eq!(format_source(&result).unwrap(), result);

    // A long chain of compatibility mutable-loan prefixes must not incur
    // quadratic suffix rewriting or accidentally duplicate the qualifier.
    let old = "mut borrow! ".repeat(256) + "foo";
    let source = format!("fnc f(): int {{\n  return {old};\n}}\n");
    let result = format_source(&source).unwrap();
    assert_eq!(result.matches("borrow! mut ").count(), 256);
    assert!(result.contains(&format!("return {}foo;", "borrow! mut ".repeat(256))), "{result}");
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn move_alias_is_canonical_and_contextual() {
    let source = "fnc f(): int {\n  return move!   foo;\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("return take! foo;"), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
    for bad in ["move!foo", "move!(foo)", "move!! foo"] {
        let source = format!("fnc f(): int {{\n  return {bad};\n}}\n");
        assert!(format_source(&source).is_err(), "{bad}");
    }
    let source = "fnc f(): bool {\n  val move = 7;\n  // move! foo\n  stdio.println(\"move! foo\");\n  return move!eq foo;\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("val move = 7;"), "{output}");
    assert!(output.contains("// move! foo"), "{output}");
    assert!(output.contains("\"move! foo\""), "{output}");
    assert!(output.contains("move!eq foo"), "{output}");
}
