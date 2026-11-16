//! Reversed continuous pipes and reverse single-value application.
//! A formatter checks spelling/layout/fixed points, not compiler graph
//! validation, stream effects, or stage execution.
use oreslang_format::{format_source, is_formatted};

fn fixed_point(input: &str) -> String {
    let once = format_source(input).expect("directional pipeline must format");
    let twice = format_source(&once).expect("formatted pipeline must format");
    assert_eq!(once, twice, "non-idempotent: {once}");
    assert!(is_formatted(&once).expect("canonical check"));
    once
}

#[test]
fn multi_line_reverse_actor_pipeline_matches_forward_layout_without_rewriting_direction() {
    let input = r#"pub fnc connect(): int {
val forward =
spawn Producer()
pipe spawn Transformer()
pipe spawn Consumer();
val reverse =
spawn Consumer()
rpipe spawn Transformer()
rpipe spawn Producer();
return 42;
}
"#;
    let output = fixed_point(input);
    assert!(output.contains(
        "  val forward =\n    spawn Producer()\n      pipe spawn Transformer()\n      pipe spawn Consumer();"
    ), "{output}");
    assert!(output.contains(
        "  val reverse =\n    spawn Consumer()\n      rpipe spawn Transformer()\n      rpipe spawn Producer();"
    ), "{output}");
    assert_eq!(output.matches("rpipe spawn").count(), 2, "{output}");
    assert_eq!(output.matches("      pipe spawn").count(), 2, "{output}");
}

#[test]
fn reverse_pipe_stage_continuations_work_after_trailing_or_leading_keyword() {
    let source = r#"fnc connect(): void {
val reverse = spawn Consumer() rpipe
spawn Transformer() rpipe
spawn Producer();
val prefixed = spawn Consumer()
rpipe spawn Transformer()
rpipe spawn Producer();
return;
}
"#;
    let output = fixed_point(source);
    for token in [
        "spawn Consumer() rpipe",
        "spawn Transformer() rpipe",
        "spawn Producer();",
        "rpipe spawn Transformer()",
        "rpipe spawn Producer();",
    ] {
        assert!(output.contains(token), "missing {token:?}: {output}");
    }
    assert!(!output.contains("|> spawn"), "{output}");
}

#[test]
fn continuous_pipes_are_directionally_distinct_even_when_textually_reversed() {
    let source = r#"fnc connect(): void {
readable
pipe transform
pipe writable;
writable
rpipe transform
rpipe readable;
return;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("  readable\n    pipe transform\n    pipe writable;"), "{output}");
    assert!(output.contains("  writable\n    rpipe transform\n    rpipe readable;"), "{output}");
    assert!(!output.contains("readable rpipe transform"), "{output}");
    assert!(!output.contains("writable pipe transform"), "{output}");
}

#[test]
fn stream_adapter_capture_in_both_directions_is_not_transformed_into_assignment() {
    // This syntax is an owned inline stream adapter *proposal* (atto-spec
    // #45). It is not an arbitrary assignment expression or one-shot I/O.
    let source = r#"fnc capture(): void {
readable
pipe transform
pipe (let results: Stream<Output> = Stream.buffered<Output>(64));
(let reversed: Stream<Output> = Stream.buffered<Output>(64))
rpipe transform
rpipe readable;
return;
}
"#;
    let output = fixed_point(source);
    for token in [
        "(let results: Stream<Output> = Stream.buffered<Output>(64))",
        "(let reversed: Stream<Output> = Stream.buffered<Output>(64))",
        "pipe (let results:",
        "rpipe transform",
        "rpipe readable;",
    ] {
        assert!(output.contains(token), "missing {token:?}: {output}");
    }
    assert!(!output.contains("<< Stream.buffered"), "{output}");
    assert!(!output.contains(">> Stream.buffered"), "{output}");
}

#[test]
fn functional_forward_and_reverse_applications_remain_distinct_from_continuous_pipes() {
    let source = r#"fnc compute(): int {
val forward = input
|> transform_a
|> transform_b;
val backward = transform_b <|
transform_a <|
input;
return backward;
}
"#;
    let output = fixed_point(source);
    for marker in [
        "  val forward = input\n    |> transform_a\n    |> transform_b;",
        "  val backward = transform_b <|\n    transform_a <|\n    input;",
    ] {
        assert!(output.contains(marker), "{output}");
    }
    assert_eq!(output.matches("|> transform").count(), 2, "{output}");
    assert_eq!(output.matches("transform_a <|").count(), 1, "{output}");
    assert!(!output.contains("rpipe transform_a"), "{output}");
}

#[test]
fn reverse_application_with_leading_operator_indents_as_a_continuation() {
    let source = r#"fnc value(): int {
val answer = transform_result
<| pending;
^ answer;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("  val answer = transform_result\n    <| pending;"), "{output}");
    assert!(output.contains("  ^ answer;"), "{output}");
}

#[test]
fn single_value_capture_is_not_misread_as_a_stream_pump() {
    let source = r#"pub susp fnc transform(): int {
val result: int <<
transform_result <|
susp |f| -> await f <|
pending;
^ result;
}
"#;
    let output = fixed_point(source);
    for marker in [
        "val result: int <<",
        "transform_result <|",
        "susp |f| -> await f <|",
        "pending;",
        "^ result;",
    ] {
        assert!(output.contains(marker), "lost one-shot pipe piece {marker}: {output}");
    }
    assert!(!output.contains("rpipe"), "{output}");
}

#[test]
fn rpipe_and_reverse_value_stages_keep_caret_and_return_expressions_open() {
    let source = r#"fnc value(): int {
^ transform_result
<| pending;
}
fnc connection(): int {
return sink
rpipe transform
rpipe source;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("^ transform_result\n    <| pending;"), "{output}");
    assert!(!output.contains("^ transform_result;\n"), "{output}");
    assert!(output.contains("return sink\n    rpipe transform\n    rpipe source;"), "{output}");
    assert!(!output.contains("return sink;\n"), "{output}");
}

#[test]
fn contextual_rpipe_names_are_not_operator_stages_or_return_continuations() {
    let source = r#"fnc names(): int {
val rpipe = 4;
rpipe = 5;
rpipe(6);
return rpipe
rpipe = 7;
return 0;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("  rpipe = 5;\n"), "{output}");
    assert!(output.contains("  rpipe(6);\n"), "{output}");
    assert!(output.contains("  return rpipe;\n  rpipe = 7;\n"), "{output}");
}

#[test]
fn quoted_text_comments_generics_and_mixed_direction_are_not_rewritten() {
    let source = r#"fnc preview(): void {
  // "rpipe pipe |> <| << >>" is only an explanation
  const string reminder = "rpipe pipe |> <| << >>";
  val Map<string, Array<int>> nested = Map.of();
  val invalid_direction = source pipe transform rpipe sink;
  return;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains(r#""rpipe pipe |> <| << >>""#), "{output}");
    assert!(output.contains("source pipe transform rpipe sink"), "{output}");
    assert!(output.contains("Map<string, Array<int>> nested"), "{output}");
    assert_eq!(output.matches("rpipe sink").count(), 1, "{output}");
    // Mixed direction is rejected by the proposed compiler contract,
    // not made valid by a source-only formatter.
}

#[test]
fn reversed_actor_chain_preserves_all_stages_at_2_3_and_16() {
    // The proposed directed-graph contract supports arbitrary checked
    // stages; formatting must keep every written stage exactly once.
    for count in [2_usize, 3, 16] {
        let mut input = String::from("fnc reversed(): int {\nval connection =\nspawn Sink0()\n");
        for n in 1..count {
            let stop = if n + 1 == count { ";" } else { "" };
            input.push_str(&format!("rpipe spawn Stage{n}(){stop}\n"));
        }
        input.push_str("return 42;\n}\n");

        let output = fixed_point(&input);
        assert!(output.contains("    spawn Sink0()\n"), "{count}: {output}");
        for n in 1..count {
            assert!(
                output.contains(&format!("      rpipe spawn Stage{n}()")),
                "{count}: missing stage {n}: {output}"
            );
        }
        assert_eq!(output.matches("rpipe spawn Stage").count(), count - 1);
        assert!(output.contains("  return 42;\n"), "{count}: {output}");
    }
}

#[test]
fn literal_or_comment_operator_text_must_not_continue_a_return() {
    let source = r#"fnc explanation(): string {
  return "literal <| rpipe |>";
  // rpipe source <| transformed
  return "finished";
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("  return \"literal <| rpipe |>\";"), "{output}");
    assert!(output.contains("  // rpipe source <| transformed"), "{output}");
    assert!(output.contains("  return \"finished\";"), "{output}");
}

#[test]
fn pipe_and_rpipe_contextual_calls_remain_callable_identifiers() {
    let source = r#"fnc names(): int {
pipe(1);
rpipe(2);
val pipe = 3;
val rpipe = 4;
return pipe + rpipe;
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("  pipe(1);"), "{output}");
    assert!(output.contains("  rpipe(2);"), "{output}");
    assert!(output.contains("  val pipe = 3;"), "{output}");
    assert!(output.contains("  val rpipe = 4;"), "{output}");
}

#[test]
fn four_repository_sha_identical_directional_sources_round_trip_without_rewrite() {
    // Same exact original .atto byte blobs committed to Java, LLVM, .NET,
    // and formatter. Failing compiler sources must remain failing sources.
    let source_set = [
        ("forward", include_str!("fixtures/directional-parity/directional-value-forward-v1.atto")),
        ("reverse", include_str!("fixtures/directional-parity/directional-value-reverse-v1.atto")),
        ("mixed", include_str!("fixtures/directional-parity/directional-value-mixed-reject.atto")),
        ("rpipe", include_str!("fixtures/directional-parity/directional-actor-rpipe-reject.atto")),
        ("identifier", include_str!("fixtures/directional-parity/directional-rpipe-contextual-name.atto")),
    ];
    for (name, original) in source_set {
        let formatted = fixed_point(original);
        match name {
            "forward" => assert!(formatted.contains("20 |> increment |> twice"), "{formatted}"),
            "reverse" => assert!(formatted.contains("twice <| increment <| 20"), "{formatted}"),
            "mixed" => assert!(formatted.contains("20 |> increment <| twice"), "{formatted}"),
            "rpipe" => assert!(formatted.contains("4 rpipe 2"), "{formatted}"),
            "identifier" => assert!(formatted.contains("val int rpipe = 42"), "{formatted}"),
            _ => unreachable!(),
        }
    }
}
