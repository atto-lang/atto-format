//! Host-only formatter regression tests for atto-spec#303.
//! Formatting does not certify compiler parsing or execution.
use oreslang_format::format_source;

fn preserve(source: &str, fragments: &[&str]) {
    let input = format!("fnc example(): void {{\n{source}\n}}\n");
    let output = format_source(&input).expect("target syntax should be preserved");
    for fragment in fragments {
        assert!(output.contains(fragment), "missing {fragment:?}: {output}");
    }
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn recursive_aggregate_delimiters_remain_distinct() {
    preserve(r#"const p = x := (1, 2);
const t = (x := 1, y := 2);
const r = (x = (1, 2), y = (3, 4));
const tr = ((x = 1), (y = 2));
const rp = (a = (x := 1), b = (y := 2));
const pr = x := (y = 2);
const pp = x := (y := 2);
const tt = ((1, 2), (3, 4));
const rr = (outer = (inner = 42));"#,
        &["x := (1, 2)", "(x := 1, y := 2)", "(x = (1, 2), y = (3, 4))",
          "((x = 1), (y = 2))", "(a = (x := 1), b = (y := 2))", "x := (y = 2)",
          "x := (y := 2)", "((1, 2), (3, 4))", "(outer = (inner = 42))"]);
}

#[test]
fn direct_quoted_record_is_not_a_named_argument() {
    preserve(r#"foo(x = 42);
foo("x" = 42);
foo(("x" = 42, y = 5));
foo("x" = 1, "y" = 2);
foo("outer" = ("inner" = 42));
foo((outer = (inner = 42)));
foo("x" := 42);
foo("x" => 42);
foo("x" == "y");"#,
        &["foo(x = 42);", r#"foo("x" = 42);"#, r#"foo(("x" = 42, y = 5));"#,
          r#"foo("x" = 1, "y" = 2);"#, r#"foo("outer" = ("inner" = 42));"#,
          "foo((outer = (inner = 42)));", r#"foo("x" := 42);"#, r#"foo("x" => 42);"#,
          r#"foo("x" == "y");"#]);
}

#[test]
fn singleton_tuple_commas_and_pair_grouping_are_preserved() {
    preserve(r#"const p = (x := 1);
const t = (x := 1,);
const r = (x = 1,);
const tr = ((x = 1),);
const tt = ((1, 2),);
const pt = x := (1,);
const rr = (outer = (inner = 42));"#,
        &["(x := 1);", "(x := 1,);", "(x = 1,);", "((x = 1),);", "((1, 2),);",
          "x := (1,);", "(outer = (inner = 42));"]);
}

#[test]
fn quoted_label_bytes_and_invalid_labels_are_not_repaired() {
    preserve(r#"foo("x-y" = 1);
foo("x " = 2);
foo("x" /* label */ = 3);
const r = ("x" = 1, "\u{78}" = 2);
const m = {"x-y": 1};
const p = "x-y" := 1;"#,
        &[r#"foo("x-y" = 1);"#, r#"foo("x " = 2);"#, r#"foo("x" /* label */ = 3);"#,
          r#"("x" = 1, "\u{78}" = 2)"#, r#"{"x-y": 1}"#, r#""x-y" := 1"#]);
}
