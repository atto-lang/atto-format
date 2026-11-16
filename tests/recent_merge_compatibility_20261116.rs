//! Merge-window formatter admission tests (2026-11-11 through 2026-11-16).
//! Source formatting/idempotence evidence only, never compiler or guest proof.
use oreslang_format::{format_source, is_formatted};

fn fixed_point(source: &str) -> String {
    let first = format_source(source).expect("format source");
    let second = format_source(&first).expect("format canonical source");
    assert_eq!(first, second, "not idempotent for:\n{source}");
    assert!(is_formatted(&first).expect("is_formatted"));
    first
}

#[test]
fn boolean_xor_continuations_do_not_turn_into_caret_returns() {
    // Java #3009 / LLVM #1728 / spec #52: && > ^^ > ||;
    // ^ remains a separately proposed return shorthand, not logical XOR.
    let source = r#"fnc choose(bool left, bool right): bool {
val a = left ^^
right;
val b = left
^^ right;
^ a ^^ b
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("  val a = left ^^\n    right;"), "{output}");
    assert!(output.contains("  val b = left\n    ^^ right;"), "{output}");
    assert!(output.contains("  ^ a ^^ b;"), "{output}");
    assert!(!output.contains("^ ^ right"), "{output}");
}

#[test]
fn multiline_caret_returns_do_not_receive_early_semicolons_before_xor() {
    let source = r#"fnc fold(bool left, bool right): bool {
^ left
^^ right;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("  ^ left\n    ^^ right;"), "{output}");
    assert!(!output.contains("^ left;\n"), "{output}");
}

#[test]
fn comment_nesting_accepts_127_and_128_levels() {
    // A single physical line is intentional: the 128 nesting threshold
    // must not depend on the formatter's indentation or line splitting.
    for depth in [1, 2, 127, 128] {
        let comment = format!("{}payload{}", "/*".repeat(depth), "*/".repeat(depth));
        let source = format!("fnc value() -> int {{\n{comment}\nreturn 42;\n}}\n");
        let output = fixed_point(&source);
        assert!(output.contains(&comment), "depth {depth}: {output}");
        assert!(output.contains("return 42;"), "depth {depth}: {output}");
    }
}

#[test]
fn comment_nesting_rejects_129th_opener_with_precise_source_line() {
    for depth in [129, 130, 256] {
        let comment = format!("{}body{}", "/*".repeat(depth), "*/".repeat(depth));
        let source = format!("fnc value() -> int {{\n{comment}\nreturn 42;\n}}\n");
        let error = format_source(&source).expect_err("must reject comment depth > 128");
        assert_eq!(error.line(), 2, "depth {depth}");
        assert_eq!(error.message(), "block comment nesting exceeds 128");
    }
}

#[test]
fn nested_comment_depth_is_carried_across_physical_lines() {
    let source = format!(
        "fnc value() -> int {{\n{}\n{}\n{}\nreturn 1;\n}}\n",
        "/*".repeat(64),
        "/*".repeat(64),
        "*/".repeat(128),
    );
    fixed_point(&source);

    let rejected = format!(
        "fnc value() -> int {{\n{}\n{}\n{}\nreturn 1;\n}}\n",
        "/*".repeat(64),
        "/*".repeat(65),
        "*/".repeat(129),
    );
    let error = format_source(&rejected).expect_err("cross-line depth 129");
    assert_eq!(error.line(), 3);
    assert_eq!(error.message(), "block comment nesting exceeds 128");
}

#[test]
fn apparent_comment_openers_inside_literals_and_line_comments_are_not_counted() {
    let delimiters = "/*".repeat(180);
    let source = format!(
        "fnc value() -> int {{\n// {delimiters}\nconst string marker = \"{delimiters}\";\n/* one */\nreturn 42;\n}}\n"
    );
    let output = fixed_point(&source);
    assert!(output.contains(&format!("\"{delimiters}\"")));
    assert!(output.contains(&format!("// {delimiters}")));
}

#[test]
fn disallowed_terminal_and_bidi_controls_fail_closed_even_in_comments() {
    // LLVM #1834/#1839 and .NET #540: reject at source preflight,
    // including lexically ignored or quoted source.
    for control in [
        '\u{1b}', '\u{7f}', '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}',
        '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}',
        '\u{2067}', '\u{2068}', '\u{2069}',
    ] {
        for wrapper in ["// ", "const string text = \""] {
            let source = if wrapper.starts_with("//") {
                format!("fnc value() -> int {{\n  // x{control}y\n  return 42;\n}}\n")
            } else {
                format!(
                    "fnc value() -> int {{\n  const string text = \"x{control}y\";\n  return 42;\n}}\n"
                )
            };
            let error = format_source(&source).expect_err("disallowed source control");
            assert_eq!(error.line(), 2, "control U+{:04X}", control as u32);
            assert!(
                error.message().contains("control"),
                "control U+{:04X}: {error}",
                control as u32
            );
        }
    }
}

#[test]
fn valid_unicode_and_ascii_controls_remain_formatable() {
    for source in [
        "fnc f() -> int {\n  const string emoji = \"😀\";\n  return 1;\n}\n",
        "fnc f() -> int {\n\tconst string multilingual = \"中文—العربية\";\nreturn 1;\n}\n",
        "fnc f() -> int {\r\n  return 1;\r\n}\r\n",
    ] {
        let output = fixed_point(source);
        assert!(output.contains("return 1;"));
    }
}

#[test]
fn newly_contextual_keywords_remain_identifiers_and_names() {
    // LLVM #1854 follows Java: these words are contextual, not globally
    // reserved. Formatter cannot rename them or insert structural blocks.
    let source = r#"fnc value(int using, int serial) -> int {
val int structural = using;
val int requires = serial;
val int pure = structural;
val int borrow = requires;
return pure + borrow;
}
"#;
    let output = fixed_point(source);
    for marker in [
        "int using, int serial",
        "int structural = using;",
        "int requires = serial;",
        "int pure = structural;",
        "int borrow = requires;",
        "return pure + borrow;",
    ] {
        assert!(output.contains(marker), "missing {marker:?}: {output}");
    }
}

#[test]
fn mixed_name_first_and_type_first_formals_keep_canonical_type_first_output() {
    // .NET #525 now admits both source forms. The formatter already
    // canonicalizes unambiguous required name-first parameters to type-first;
    // do not silently change the established formatter contract.
    let source = r#"fnc sum(x: int, int y) -> int {
return x + y;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("sum(int x, int y)"), "{output}");
    assert!(!output.contains("sum(x: int, int y)"), "{output}");
}

#[test]
fn multiline_actor_chain_and_boolean_xor_in_one_callable_do_not_cross_contaminate() {
    let source = r#"fnc run() -> int {
val connection =
spawn Producer()
pipe spawn Transformer()
pipe spawn Consumer();
val ready = foo ^^
bar;
^ 42
}
"#;
    let output = fixed_point(source);
    assert!(
        output.contains("    spawn Producer()\n      pipe spawn Transformer()\n      pipe spawn Consumer();"),
        "{output}"
    );
    assert!(output.contains("  val ready = foo ^^\n    bar;"), "{output}");
    assert!(output.contains("  ^ 42;"), "{output}");
}
