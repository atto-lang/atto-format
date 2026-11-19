// HOST-ONLY formatter consistency. Not a compiler, semantic AST or guest proof.
use oreslang_format::format_source;

const CONSUMER: &str = include_str!("../contracts/v1/atto-spec-consumer-v1.json");

#[test]
fn spec_consumer_snapshot_advertises_no_unearned_guest_parity() {
    for phrase in [
        "atto-lang/atto-spec",
        "atto-lang/atto-format",
        "ATTO-OWN-003",
        "ATTO-FMT-002",
        "ATTO-FMT-003",
        "semantic_ast_equivalence",
        "not_executed",
    ] {
        assert!(CONSUMER.contains(phrase), "missing registry field: {phrase}");
    }
}

#[test]
fn ownership_spellings_remain_distinct_and_formatting_is_idempotent() {
    let source = "fnc demo() -> void {\nval a = copy! owner;\nval b = copy!! owner;\nval c = take! owner;\ndrop! owner;\nreturn;\n}\n";
    let formatted = format_source(source).expect("lexical formatter should accept sample");
    assert!(formatted.contains("copy! owner;"));
    assert!(formatted.contains("copy!! owner;"));
    assert!(formatted.contains("take! owner;"));
    assert!(formatted.contains("drop! owner;"));
    assert_eq!(format_source(&formatted).expect("second format"), formatted);
}
