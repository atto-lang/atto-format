use oreslang_format::{format_source, is_formatted};

#[test]
fn canonicalizes_spaced_compact_and_legacy_ownership_prefixes() {
    for (input, expected) in [
        ("copy!foo", "copy! foo"),
        ("copy!   foo", "copy! foo"),
        ("rt copy foo", "copy! foo"),
        ("take!foo", "take! foo"),
        ("rt take foo", "take! foo"),
        ("borrow!foo.member", "borrow! foo.member"),
        ("rt borrow foo", "borrow! foo"),
        ("mut borrow!foo", "mut borrow! foo"),
        ("rt mut borrow foo", "mut borrow! foo"),
        ("rt borrow mut foo", "mut borrow! foo"),
        ("share!foo", "share! foo"),
        ("rt share foo", "share! foo"),
        ("copy!(foo + bar)", "copy! (foo + bar)"),
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
    assert!(output.contains("return !foo;"), "{output}");
}

#[test]
fn never_rewrites_literals_or_comments_or_ownership_like_identifiers() {
    let source = "fnc f(): void {\n  stdio.println(\"rt take foo, copy!foo\");\n  // rt borrow mut foo, share!bar\n  val copied = 1;\n  val view = rt /* preserved */ copy foo;\n  return;\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("\"rt take foo, copy!foo\""));
    assert!(output.contains("// rt borrow mut foo, share!bar"));
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
    let nested = "copy!".repeat(256) + "foo";
    let source = format!("fnc f(): int {{\n  return {nested};\n}}\n");
    let result = format_source(&source).unwrap();
    assert_eq!(result.matches("copy! ").count(), 256);
    assert_eq!(format_source(&result).unwrap(), result);

    let legacy = "rt copy ".repeat(256) + "foo";
    let source = format!("fnc f(): int {{\n  return {legacy};\n}}\n");
    let result = format_source(&source).unwrap();
    assert_eq!(result.matches("copy! ").count(), 256);
    assert_eq!(format_source(&result).unwrap(), result);
}
