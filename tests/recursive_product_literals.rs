//! Host-only formatter regressions for revised atto-spec#303.
//! Text preservation does not certify parser equivalence or guest execution.
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
fn declaration_context_and_empty_aggregate_identity() {
    preserve(r#"const x = ();
const Record empty = ();
const Record empty2 = {};
const p = (five := 5);
const t = (five := 5, six := 8);
const r = (five: 5, six: 8);
const Record explicit_r = (five := 5, six := 8);
const Record explicit_colon = (a: 5);
const Record explicit_equal = (a = 5);
const r2 = {five := 5, six = 8};
const Map<string,int> m = {};"#,
        &["const x = ();", "const Record empty = ();", "const Record empty2 = {};",
          "(five := 5);", "(five := 5, six := 8);", "(five: 5, six: 8);",
          "const Record explicit_r = (five := 5, six := 8);",
          "(a: 5);", "(a = 5);", "{five := 5, six = 8}", "m = {};"]);
}

#[test]
fn recursive_aggregate_delimiters_remain_distinct() {
    preserve(r#"const p = x := (1, 2);
const t = (x := 1, y := 2);
const r = {x: (1, 2), y: (3, 4)};
const tr = ({x: 1}, {y: 2});
const rp = {a: (x := 1), b: (y := 2)};
const pr = x := {y: 2};
const pp = x := (y := 2);
const tt = ((1, 2), (3, 4));
const rr = {outer: {inner: 42}};"#,
        &["x := (1, 2)", "(x := 1, y := 2)", "{x: (1, 2), y: (3, 4)}",
          "({x: 1}, {y: 2})", "{a: (x := 1), b: (y := 2)}", "x := {y: 2}",
          "x := (y := 2)", "((1, 2), (3, 4))", "{outer: {inner: 42}}"]);
}

#[test]
fn calls_keep_brace_records_and_named_arguments_distinct() {
    preserve(r#"foo(x = 42);
foo("x" = 42);
foo(x: 42);
foo({x: 42});
foo({x := 42});
foo({x = 42});
foo({"x": 42, y := 5});
foo({x: 1}, {y: 2});
foo({x: 1, y: 2});
foo("outer" = {inner: 42});
foo(x := 42);
foo(x := 42, y := 5);
foo(x := 42, n = 5);
foo((x := 42));
foo((x := 42,));
foo((x := 42, y := 5));
foo("x" => 42);"#,
        &["foo(x = 42);", r#"foo("x" = 42);"#, "foo(x: 42);",
          "foo({x: 42});", "foo({x := 42});", "foo({x = 42});",
          r#"foo({"x": 42, y := 5});"#, "foo({x: 1}, {y: 2});",
          "foo({x: 1, y: 2});", r#"foo("outer" = {inner: 42});"#,
          "foo(x := 42);", "foo(x := 42, y := 5);", "foo(x := 42, n = 5);",
          "foo((x := 42));", "foo((x := 42,));", "foo((x := 42, y := 5));",
          r#"foo("x" => 42);"#]);
}

#[test]
fn quoted_record_fields_and_arrow_map_keys_remain_distinct() {
    preserve(r#"const r = {"name": "Alice"};
const m = {"name" => "Alice"};
const m2 = {"x-y" => 1};
const p = "x-y" := 1;
const t = ({x: 1},);
const r2 = {"x" /* label */ : 42};"#,
        &[r#"{"name": "Alice"}"#, r#"{"name" => "Alice"}"#, r#"{"x-y" => 1}"#,
          r#""x-y" := 1"#, "({x: 1},);", r#"{"x" /* label */ : 42}"#]);
}

#[test]
fn portable_call_boundaries_and_unresolved_grouped_forms_survive_formatting() {
    // Text preservation only: grouped forms are deliberately not assigned
    // portable call semantics by the formatter.
    preserve(r#"foo(x = 4, y = 8);
foo({x = 4, y = 8});
foo(data = {x = 4, y = 8});
foo((x = 4, y = 8));
foo((x: 4, y: 8));"#,
        &["foo(x = 4, y = 8);", "foo({x = 4, y = 8});",
          "foo(data = {x = 4, y = 8});", "foo((x = 4, y = 8));",
          "foo((x: 4, y: 8));"]);
}
