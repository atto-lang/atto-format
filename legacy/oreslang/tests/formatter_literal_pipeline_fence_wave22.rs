//! Source formatter lexical boundaries only; not a guest compiler test.
use oreslang_format::{format_source, is_formatted};

fn fixed_point(source: &str) -> String {
    let formatted = format_source(source).expect("valid source");
    let repeated = format_source(&formatted).expect("valid formatted source");
    assert_eq!(formatted, repeated, "formatter must reach a fixed point");
    assert!(is_formatted(&formatted).expect("canonical formatting check"));
    formatted
}

#[test]
fn url_and_comment_sentinels_inside_strings_remain_literal() {
    let source = r#"fnc label() -> String {
return "https://example.org/a//b?if=then;fi";
}
"#;
    let output = fixed_point(source);
    assert!(output.contains(r#""https://example.org/a//b?if=then;fi""#));
}

#[test]
fn adjacent_comments_do_not_rewrite_neighboring_function_bodies() {
    let source = r#"// if ready; then return 19; fi
// critical-read actor select over end
fnc first() -> int {
return 19;
}
// if ready; then return 23; fi
fnc second() -> int {
return 23;
}
"#;
    let output = fixed_point(source);
    for marker in [
        "// if ready; then return 19; fi",
        "// critical-read actor select over end",
        "// if ready; then return 23; fi",
        "return 19;",
        "return 23;",
    ] {
        assert!(output.contains(marker), "lost lexical marker: {marker}");
    }
}

#[test]
fn unrelated_numeric_literals_remain_stable_across_multiple_functions() {
    for index in 0..32_u64 {
        let left = index * 101 + 7;
        let right = index * 103 + 11;
        let source = format!(
            "fnc left() -> int {{\nreturn {left};\n}}\nfnc right() -> int {{\nreturn {right};\n}}\n"
        );
        let output = fixed_point(&source);
        assert!(output.contains(&format!("return {left};")), "index {index}");
        assert!(
            output.contains(&format!("return {right};")),
            "index {index}"
        );
    }
}
