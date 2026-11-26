use oreslang_format::format_source;

#[test]
fn named_aggregate_type_binders_survive_roundtrip() {
    let source = r#"fnc main(): int {
const Record<infer T1> r = (a=true, b="xyz", c=5);
const Tuple<infer T2> t = (true, "xyz", 5);
const struct<infer T3> s = {a:true, b:"xyz", c:5};
const Record<T1> r2 = (a=false, b="abc", c=10);
const Tuple<T2> t2 = (false, "abc", 10);
const struct<T3> s2 = {a:false, b:"abc", c:10};
return 0;
}
"#;
    let output = format_source(source).expect("named inference should format");
    for marker in ["Record<infer T1>", "Tuple<infer T2>", "struct<infer T3>",
                   "Record<T1>", "Tuple<T2>", "struct<T3>"] {
        assert!(output.contains(marker), "lost {marker}: {output}");
    }
    assert_eq!(format_source(&output).unwrap(), output);
}
