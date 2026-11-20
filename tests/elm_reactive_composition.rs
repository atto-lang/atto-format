use oreslang_format::{format_source, is_formatted};

#[test]
fn snake_case_composition_chains_survive_formatting() {
    let source = r#"
fnc option_pipe(Option<int> input): Option<int> {
  return input.flat_map<int>(|int n| -> { return Some(n + 1); }).and_then<int>(|int n| -> { return Some(n * 2); });
}

fnc result_pipe(Result<int, string> input): Result<int, string> {
  return input.map_err<string>(|string err| -> { return err; }).flat_map<int>(|int n| -> { return Ok(n); });
}

fnc future_pipe(Future<int> input): Future<int> {
  return input.flat_map(|int n| -> { return next_future(n); }).on_success(|int n| -> { stdio.println(n); return; });
}

fnc observable_pipe(Observable<int> input): Observable<int> {
  return rx_channels.filter_map(input, |int n| -> { return Some(n); });
}
"#;
    let once = format_source(source).expect("composition fixture should format");
    for marker in [
        ".flat_map<int>(",
        ".and_then<int>(",
        ".map_err<string>(",
        ".flat_map(",
        ".on_success(",
        "rx_channels.filter_map(",
    ] {
        assert!(once.contains(marker), "lost {marker}: {once}");
    }
    let twice = format_source(&once).expect("format must be idempotent");
    assert_eq!(once, twice);
    assert!(is_formatted(&twice).unwrap());
}

#[test]
fn legacy_future_camel_case_names_remain_source_compatible() {
    let source = r#"fnc old_future(Future<int> f): Future<int> {
  return f.flatMap(|int n| -> { return next_future(n); }).onSuccess(|int n| -> { return; });
}
"#;
    let once = format_source(source).expect("legacy Future source should format");
    assert!(once.contains(".flatMap("), "{once}");
    assert!(once.contains(".onSuccess("), "{once}");
    assert_eq!(once, format_source(&once).unwrap());
}

#[test]
fn existing_ternary_and_as_question_cast_do_not_become_postfix_propagation() {
    let source = r#"fnc ternary(bool valid): int {
  return valid ? 3 : 4;
}
fnc optional_cast(Any v): Option<int> {
  return v as? int;
}
"#;
    let once = format_source(source).expect("existing valid question syntax should format");
    assert!(once.contains("?"), "ternary delimiter lost: {once}");
    assert!(once.contains(":"), "ternary alternative lost: {once}");
    assert!(
        once.contains("as? int"),
        "optional cast was rewritten: {once}"
    );
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn postfix_question_mark_preserves_typed_short_circuit_and_ternary() {
    let source = r#"fnc forward(Result<int, String> incoming): Result<int, String> {
  val int n = incoming?;
  return Ok(n);
}
fnc optional(Option<int> incoming): Option<int> {
  val int n = incoming?;
  return Some(n);
}
fnc choice(bool flag): int {
  return flag?1:2;
}
"#;
    let once =
        format_source(source).expect("postfix ? must be preserved or format must fail closed");
    for fragment in ["incoming?;", "flag", "?", ":"] {
        assert!(
            once.contains(fragment),
            "question operator lost {fragment}: {once}"
        );
    }
    assert_eq!(format_source(&once).unwrap(), once);
}
