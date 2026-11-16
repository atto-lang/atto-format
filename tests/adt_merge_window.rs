//! ADT/enum/structural source fixtures from independently merged compiler PRs.
//! These tests assert formatter preservation and idempotence, NOT typechecking,
//! guest execution, actor transfer, or backend parity.
use oreslang_format::{format_source, is_formatted};

fn fixed_point(source: &str) -> String {
    let first = format_source(source).expect("formatter must accept the source");
    let second = format_source(&first).expect("formatter must accept canonical output");
    assert_eq!(first, second, "non-idempotent output:\n{first}");
    assert!(is_formatted(&first).unwrap(), "not canonical: {first}");
    first
}

const MERGED_ADT_FIXTURES: &[(&str, &str)] = &[
    ("java-numeric-sum-equality", include_str!("fixtures/adt-merge-window/java-numeric-sum-equality.atto")),
    ("net-nullary-enum", include_str!("fixtures/adt-merge-window/net-nullary-enum.atto")),
    ("net-enum-function", include_str!("fixtures/adt-merge-window/net-enum-function.atto")),
    ("rs-result-ok", include_str!("fixtures/adt-merge-window/rs-result-ok.atto")),
    ("rs-result-err", include_str!("fixtures/adt-merge-window/rs-result-err.atto")),
    ("rs-option-some", include_str!("fixtures/adt-merge-window/rs-option-some.atto")),
    ("rs-option-none", include_str!("fixtures/adt-merge-window/rs-option-none.atto")),
    ("rs-nested-option-result-ok", include_str!("fixtures/adt-merge-window/rs-nested-option-result-ok.atto")),
    ("rs-nested-option-result-err", include_str!("fixtures/adt-merge-window/rs-nested-option-result-err.atto")),
    ("rs-nested-option-result-none", include_str!("fixtures/adt-merge-window/rs-nested-option-result-none.atto")),
    ("rs-nested-option-result-arguments", include_str!("fixtures/adt-merge-window/rs-nested-option-result-arguments.atto")),
];

#[test]
fn merged_java_net_and_rust_wasm_adt_fixtures_all_round_trip() {
    for &(name, source) in MERGED_ADT_FIXTURES {
        let result = fixed_point(source);
        // Never strip constructor names, enum tags, nominal owners or ADT
        // payload types from independently compiled input.
        for marker in ["Option<Result<int", "Some(", "Err(", "on None", "on Light."]
            .into_iter()
            .filter(|marker| source.contains(**marker))
        {
            assert!(result.contains(marker), "{name}: lost {marker:?}:\n{result}");
        }
    }
}

#[test]
fn enum_variant_values_and_exhaustive_arms_keep_nominal_identity() {
    for name in ["net-nullary-enum", "net-enum-function"] {
        let source = MERGED_ADT_FIXTURES.iter().find(|(n, _)| *n == name).unwrap().1;
        let output = fixed_point(source);
        for marker in [
            "define enum Light as", "Red = 1", "Green = 2",
            "Yellow = 3", "on Light.Red", "on Light.Green", "on Light.Yellow",
        ] {
            assert!(output.contains(marker), "{name}: {marker}: {output}");
        }
        assert!(!output.contains("on Red ->"), "{name}: {output}");
    }
}

#[test]
fn option_result_tags_and_nested_option_are_never_flattened() {
    let source = include_str!("fixtures/adt-merge-window/java-numeric-sum-equality.atto");
    let output = fixed_point(source);
    for marker in [
        "Option<Result<int, String>>", "Some(Ok(5))",
        "Some(Err(\"5\"))", "Option<Option<int>>",
        "Some(None)", "left eq equal", "inner_none neq outer_none",
    ] {
        assert!(output.contains(marker), "lost tagged sum value {marker}: {output}");
    }
    assert!(!output.contains("Option<int> inner_none"), "{output}");
}

#[test]
fn guarded_structural_record_patterns_are_distinct_from_executable_arm_braces() {
    let source = r#"fnc handle() -> int {
  match incoming over {
    on ~{x?: 10, y: _} as val p when validate(p) -> { use(p); },
    on ~{x: 11, y: _} when ready() -> { accept(); };
    on _ -> { fallback(); }
  }
  return 0;
}
"#;
    let output = fixed_point(source);
    for marker in [
        "on ~{x?: 10, y: _} as val p when validate(p) -> {",
        "on ~{x: 11, y: _} when ready() -> {",
        "on _ -> {",
    ] {
        assert!(output.contains(marker), "lost guarded shape {marker}: {output}");
    }
    assert!(!output.contains("on ~{\n"), "record pattern opened a body: {output}");
}

#[test]
fn adt_nested_structural_patterns_and_exact_value_alternatives_are_opaque() {
    let source = r#"fnc choose() -> int {
  match incoming over {
    on Some(~{foo: "dog", count?: _, nested: {active: true}}) -> { return 1; },
    on Ok(42) when ready() -> { return 2; },
    on Err(_) -> { return 3; },
    on _ -> { return 0; }
  }
}
"#;
    let output = fixed_point(source);
    for marker in [
        r#"foo: "dog""#, "count?: _", "nested: {active: true}",
        "on Ok(42) when ready()", "on Err(_)", "on _ ->",
    ] {
        assert!(output.contains(marker), "lost pattern {marker}: {output}");
    }
}

#[test]
fn sequence_pattern_rest_wildcards_keep_direction_and_explicit_binders() {
    // atto-spec #16: reference/spec syntax, NOT universal compiler admission.
    for pattern in [
        "List.of(..._, 5 as val x, 3 as val y, 2 as val z, _...)",
        "Tuple.of(1, _, 3)",
        "List.of(_..., 7, ..._)",
    ] {
        let source = format!(
            "fnc check() -> int {{\n  match value over\n    on {pattern} -> {{ return 1; }}\n    on _ -> {{ return 0; }}\n  end\n}}\n"
        );
        let output = fixed_point(&source);
        assert!(output.contains(pattern), "mutated rest/discard pattern: {output}");
        assert!(!output.contains("on default"), "changed match catch-all: {output}");
    }
}

#[test]
fn explicit_type_patterns_and_structural_union_formals_do_not_change_meaning() {
    let source = r#"fnc classify({foo: string | int, enabled?: bool} value) -> int {
  match value over {
    on is !int -> { return 1; },
    on is int as val v -> { return 2; },
    on _ -> { return 0; }
  }
}
"#;
    let output = fixed_point(source);
    for marker in [
        "{foo: string | int, enabled?: bool} value",
        "on is !int ->", "on is int as val v ->", "on _ ->",
    ] {
        assert!(output.contains(marker), "lost type/union marker {marker}: {output}");
    }
}

#[test]
fn nullable_presence_and_inferred_wildcard_typing_do_not_become_nominal() {
    let source = r#"fnc check({foo: string} row) -> int {
  match row over {
    on ~{foo?: "dog", bar: _} as val found -> { return 1; },
    on ~{foo: _} -> { return 2; },
    on _ -> { return 0; }
  }
}
"#;
    let output = fixed_point(source);
    assert!(output.contains("foo?: \"dog\""), "{output}");
    assert!(output.contains("bar: _"), "{output}");
    assert!(output.contains("{foo: string} row"), "{output}");
}

#[test]
fn caret_return_wrapping_end_delimited_adt_match_keeps_matching_end() {
    // ^ remains forward-looking syntax; merged Java/LLVM reject single ^.
    let source = r#"fnc classify() -> int {
^ match<int> value over
on Some(Ok(v)) -> { ^ v; }
on Some(Err(_)) -> { ^ -1; }
on None -> { ^ 0; }
end;
}
"#;
    let output = fixed_point(source);
    for marker in ["^ match<int> value over", "on Some(Ok(v))", "on Some(Err(_))", "on None", "end;"] {
        assert!(output.contains(marker), "{marker}: {output}");
    }
}

#[test]
fn typed_actor_message_adt_declarations_do_not_turn_into_control_messages() {
    // Formatter-only design compatibility; not actor/mailbox typed ABI proof.
    let source = r#"declare actor Worker receives Result<int, string> emits Option<int>;
define actor Worker receives Result<int, string> emits Option<int> as
  receive(Result<int, string> message): void {
    match message over
      on Ok(value) -> { self.send(Some(value)); }
      on Err(error) -> { self.send(None); }
    end
  }
end
"#;
    let output = fixed_point(source);
    assert!(output.contains("declare actor Worker receives Result<int, string> emits Option<int>;"), "{output}");
    assert!(output.contains("define actor Worker receives Result<int, string> emits Option<int> as"), "{output}");
    assert!(output.contains("on Ok(value)"), "{output}");
    assert!(output.contains("on Err(error)"), "{output}");
}
