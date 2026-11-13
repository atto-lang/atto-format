//! Actor-pipeline alias spelling is a source formatting contract only.
//! Formatter success must never be reported as compiler or guest parity.
use oreslang_format::{format_source, is_formatted};

#[test]
fn actor_pipeline_keyword_and_symbol_are_preserved_independently() {
    let source = r#"pub fnc main(): int {
  val symbols = spawn Parse() | spawn Validate() | spawn Sink();
  symbols.send(41); // symbolic chain
  val keywords = spawn Parse() pipe spawn Validate() pipe spawn Sink();
  keywords.send(41); // keyword chain
  return 0;
}
"#;
    let first = format_source(source).expect("source format");
    let second = format_source(&first).expect("canonical round trip");
    assert_eq!(first, second, "formatter must be idempotent");
    assert!(is_formatted(&first).expect("format check"));

    // Do not normalize actor pipes into functional |>, boolean ||, or
    // identifiers. The two independent source expressions must survive.
    assert!(
        first.contains("spawn Parse() | spawn Validate() | spawn Sink();"),
        "symbolic actor pipeline was rewritten: {first}"
    );
    assert!(
        first.contains("spawn Parse() pipe spawn Validate() pipe spawn Sink();"),
        "keyword actor pipeline was rewritten: {first}"
    );
    for marker in [
        "symbols.send(41);",
        "keywords.send(41);",
        "// symbolic chain",
        "// keyword chain",
    ] {
        assert!(first.contains(marker), "lost marker {marker}: {first}");
    }
}

#[test]
fn actor_pipe_words_in_comments_and_string_literals_are_not_operators() {
    let source = r#"pub fnc literal(): String {
  // spawn A() pipe spawn B() | spawn C()
  return "pipe | |> spawn A()"; 
}
"#;
    let output = format_source(source).expect("format source");
    assert!(output.contains("// spawn A() pipe spawn B() | spawn C()"));
    assert!(output.contains("\"pipe | |> spawn A()\""));
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn split_actor_pipe_keyword_continuations_preserve_stage_order() {
    let source = r#"pub fnc main(): int {
  val chained = spawn Parse() pipe
  spawn Validate() pipe
  spawn Sink();
  val prefixed = spawn Parse()
  pipe spawn Validate()
  pipe spawn Sink();
  return 0;
}
"#;
    let output = format_source(source).expect("multiline actor pipe");
    assert_eq!(format_source(&output).unwrap(), output);
    for marker in [
        "spawn Parse() pipe",
        "spawn Validate() pipe",
        "spawn Sink();",
        "pipe spawn Validate()",
        "pipe spawn Sink();",
    ] {
        assert!(output.contains(marker), "lost pipeline stage {marker}: {output}");
    }
}
