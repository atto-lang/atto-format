use oreslang_format::{format_source, is_formatted};

#[test]
fn end_delimited_match_keeps_canonical_on_wildcard() {
    let source = r#"fnc handle(Option<int> x): void {
do match x over
on Some(v) -> { consume(v); }
on _ -> { return; }
end
}
"#;
    let result = format_source(source).unwrap();
    assert!(result.contains("on Some(v) -> {"), "{result}");
    assert!(result.contains("on _ -> {"), "{result}");
    assert!(!result.contains("default ->"), "{result}");
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn braced_match_does_not_confuse_record_projection_with_arm_body() {
    let source = r#"do match value over {
on Foo as {v} -> { process(v); }
on _ -> { return; }
}
"#;
    let result = format_source(source).unwrap();
    assert!(result.contains("on Foo as {v} -> {"), "{result}");
    assert!(result.contains("on _ -> {"), "{result}");
    assert!(!result.contains("default ->"), "{result}");
    assert_eq!(format_source(&result).unwrap(), result);
    assert!(is_formatted(&result).unwrap());
}

#[test]
fn match_legacy_on_default_migrates_to_on_wildcard_not_switch_default() {
    let source = r#"match value over {
on default -> { return; }
}
"#;
    let result = format_source(source).unwrap();
    assert!(result.contains("on _ -> {"), "{result}");
    assert!(!result.contains("on default"), "{result}");
    assert!(!result.contains("\n  default"), "{result}");
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn switch_underscore_fallback_still_becomes_default() {
    let source = r#"switch value {
_ -> { return 0; }
case 1 -> { return 1; }
}
"#;
    let result = format_source(source).unwrap();
    assert!(result.contains("default -> {"), "{result}");
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn actor_send_result_match_preserves_on_for_ok_err_and_wildcard() {
    // The formatter is syntactic: no claim that the guest send return type
    // has already been lowered by an LLVM or Wasm compiler.
    let source = r#"pub fnc handle_send(ActorRef<Worker> worker): void {
  val result = worker.send(42);
  match result over
    on Ok(ack) -> { accepted(ack); }
    on Err(err) -> { rejected(err); }
  end
  match result over
    on Ok(ack) -> { accepted(ack); }
    on _ -> { ignored(); }
  end
}
"#;
    let formatted = format_source(source).expect("canonical on-arm formatting");
    assert!(formatted.contains("on Ok(ack) -> {"), "{formatted}");
    assert!(formatted.contains("on Err(err) -> {"), "{formatted}");
    assert!(formatted.contains("on _ -> {"), "{formatted}");
    assert!(!formatted.contains("default ->"), "{formatted}");
    assert!(!formatted.contains("\n    Ok(ack) ->"), "{formatted}");
    assert!(!formatted.contains("\n    Err(err) ->"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
