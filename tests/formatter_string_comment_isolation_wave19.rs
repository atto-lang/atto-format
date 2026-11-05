//! Formatter regression only; no compiler or guest execution is implied.
use oreslang_format::{format_source,is_formatted};

fn fixed_point(source: &str) -> String {
    let once=format_source(source).expect("valid formatter syntax");
    let twice=format_source(&once).expect("canonical syntax");
    assert_eq!(once,twice,"formatter must converge after one pass");
    assert!(is_formatted(&once).expect("validate canonical source"));
    once
}

#[test]
fn keywords_and_comment_delimiters_inside_literals_are_preserved() {
    let src=r#"fnc literal() -> String {
return "if then else fi // not a comment";
}
"#;
    let out=fixed_point(src);
    assert!(out.contains(r#""if then else fi // not a comment""#));
}

#[test]
fn comments_with_legacy_keywords_are_not_rewritten_as_executable_blocks() {
    let src=r#"// if ready; then return 7; fi
fnc value() -> int {
  // actor critical-read and switch case over end
  return 7;
}
"#;
    let out=fixed_point(src);
    assert!(out.contains("// if ready; then return 7; fi"));
    assert!(out.contains("// actor critical-read and switch case over end"));
    assert!(out.contains("return 7;"));
}

#[test]
fn canonical_function_arrows_preserve_expression_lambda_arrows() {
    let src=r#"fnc demo() => void {
val Fnc<int, int> double = |value| -> value * 2;
return;
}
"#;
    let out=fixed_point(src);
    assert!(out.contains("fnc demo() -> void"));
    assert!(out.contains("|value| -> value * 2;"));
}

#[test]
fn seeded_numeric_literals_reach_a_fixed_point_without_rewriting_values() {
    for i in 0..64_u64 {
        let value=i.wrapping_mul(1031).wrapping_add(17);
        let src=format!("fnc value() -> int {{\nreturn {value};\n}}\n");
        let out=fixed_point(&src);
        assert!(out.contains(&format!("return {value};")), "iteration {i}: {out}");
    }
}
