use oreslang_format::format_source;
#[test]
fn compact_bang_keywords_are_not_silently_repaired() {
    for keyword in ["trap!", "throw!", "raise!", "panic!", "copy!", "copy!!",
        "take!", "borrow!", "share!", "pin!", "drop!", "pipe!", "rpipe!", "stop!", "expand!"] {
        for tail in ["value", "(42)", "/*comment*/ value", "\u{a0}value"] {
            let source = format!("fnc f(): int {{\n return {keyword}{tail};\n}}\n");
            let error = format_source(&source).expect_err(&source);
            assert!(error.message().contains("E_BANG_KEYWORD_WHITESPACE"), "{source}: {error}");
        }
    }
}
#[test]
fn spaced_keyword_and_opaque_text_are_not_rejected() {
    let source = "fnc f(): int {\n // trap!(42)\n val text = \"copy!!value\";\n return trap! (42);\n}\n";
    let out = format_source(source).unwrap();
    assert!(out.contains("trap! (42)"), "{out}");
    assert!(out.contains("// trap!(42)"));
    assert!(out.contains("\"copy!!value\""));
}
