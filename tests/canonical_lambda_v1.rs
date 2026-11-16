//! Canonical-lambda source-fidelity checks only. Formatting is NOT parser,
//! borrow-checker, AOT compiler or guest-runtime certification.
use oreslang_format::{format_source, is_formatted};

fn fixed_point(source: &str) -> String {
    let once = format_source(source).expect("canonical lambda source should format");
    let twice = format_source(&once).expect("formatted lambda source should reformat");
    assert_eq!(once, twice, "formatting must be idempotent");
    assert!(is_formatted(&once).expect("formatter check must succeed"));
    once
}

#[test]
fn inferred_lambda_parameters_stay_named_in_functional_pipe() {
    let src = r#"fnc apply() -> int {
  return 21 |> (|x| -> x * 2);
}
"#;
    let output = fixed_point(src);
    assert!(output.contains("|x| -> x * 2"), "{output}");
    assert!(output.contains("|>"), "{output}");
    assert!(!output.contains("& &1"), "{output}");
    assert!(!output.contains("_1 *"), "{output}");
}

#[test]
fn explicitly_typed_and_nested_lambdas_keep_binders_and_arrows() {
    let src = r#"fnc apply() -> int {
  return (|int x| -> (|int y| -> x + y)(22))(20);
}
"#;
    let output = fixed_point(src);
    assert!(output.contains("|int x| ->"), "{output}");
    assert!(output.contains("|int y| ->"), "{output}");
    assert!(output.contains("x + y"), "{output}");
}

#[test]
fn lambda_markers_inside_strings_and_comments_are_not_transformed() {
    let src = r#"fnc show() -> String {
  // Rejected language shorthand: & &1.active / _1.name
  val String label = "|x| -> x + 1; & &1.name";
  return label;
}
"#;
    let output = fixed_point(src);
    assert!(output.contains("// Rejected language shorthand: & &1.active / _1.name"), "{output}");
    assert!(output.contains(r#""|x| -> x + 1; & &1.name""#), "{output}");
}
