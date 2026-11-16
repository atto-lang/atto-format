//! Formatter/CLI syntax contracts, not compiler or runtime execution proof.
use oreslang_format::{format_source, is_formatted};

const CANONICAL: &str = include_str!("fixtures/multiline-actor-io-return.atto");

fn fixed_point(source: &str) -> String {
    let formatted = format_source(source).expect("format source");
    assert_eq!(format_source(&formatted).unwrap(), formatted, "idempotence");
    assert!(is_formatted(&formatted).unwrap());
    formatted
}

#[test]
fn split_binding_actor_pipeline_canonicalizes_stage_indentation() {
    let source = r#"pub fnc connect() -> int {
val connection =
spawn Producer()
pipe spawn Transformer()
pipe spawn Consumer();
stdio.out << "ready";
stdio.in >> val pending_line;
^ 42
}
"#;
    assert_eq!(fixed_point(source), CANONICAL);
}

#[test]
fn operator_tokens_and_keyword_pipe_are_not_exchanged() {
    let source = r#"fnc operators() -> int {
val connection =
spawn Producer()
pipe spawn Transformer()
pipe spawn Consumer();
sink << connection;
source >> val item;
^ item
}
"#;
    let output = fixed_point(source);
    for expected in [
        "val connection =\n",
        "    spawn Producer()\n",
        "      pipe spawn Transformer()\n",
        "      pipe spawn Consumer();\n",
        "  sink << connection;\n",
        "  source >> val item;\n",
        "  ^ item;\n",
    ] {
        assert!(output.contains(expected), "lost {expected:?} in {output}");
    }
    assert!(!output.contains("|> spawn"));
    assert!(!output.contains("return item"));
}

#[test]
fn multiline_directional_operations_and_caret_returns_do_not_terminate_early() {
    let source = r#"fnc forward() -> int {
  stdio.out <<
    "hello";
  stdio.in
    >> val pending_line;
  ^ pending_line
    |> transform_result;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("stdio.out <<\n"), "{output}");
    assert!(output.contains("    \"hello\";"), "{output}");
    assert!(output.contains("stdio.in\n    >> val pending_line;"), "{output}");
    assert!(output.contains("^ pending_line\n    |> transform_result;"), "{output}");
    assert!(!output.contains("^ pending_line;\n"), "{output}");
}

#[test]
fn comments_strings_and_generic_type_closers_are_opaque() {
    let source = r#"fnc examples() -> int {
  // >> << ^ pipe spawn Fake()
  const string punctuation = ">> << ^ pipe spawn Fake()";
  val Map<string, Array<int>> nested = Map.of();
  ^ 42; // ^ 99; >> fake
}
"#;
    let output = fixed_point(source);
    for expected in [
        "// >> << ^ pipe spawn Fake()",
        "Map<string, Array<int>> nested = Map.of();",
        "^ 42; // ^ 99; >> fake",
    ] {
        assert!(output.contains(expected), "missing {expected:?}: {output}");
    }
    assert!(output.contains(r#"">> << ^ pipe spawn Fake()""#), "{output}");
}

#[test]
fn actor_pipeline_state_does_not_leak_into_other_statements() {
    let source = r#"fnc two() -> int {
  val chain =
    spawn A()
    pipe spawn B();
  source
    pipe transform;
  ^ 7;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("    spawn A()\n      pipe spawn B();"), "{output}");
    assert!(output.contains("  source\n    pipe transform;"), "{output}");
}
