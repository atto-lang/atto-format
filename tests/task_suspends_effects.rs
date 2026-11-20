use oreslang_format::format_source;

#[test]
fn suspend_spelling_is_preserved_while_modifiers_are_reordered() {
    // `suspend` aliases `suspends` in the Java grammar, but LLVM gives a
    // top-level `suspend fnc` its own cooperative-loop meaning (which needs a
    // `:` return separator) and rejects `suspends fnc`, so the spelling is
    // never rewritten and the separator is the one both compilers accept.
    let source = "suspend pub fnc load() -> int {\n  return 7;\n}\n";
    let output = format_source(source).expect("format");
    assert!(output.contains("pub suspend fnc load(): int {"), "{output}");
    assert_eq!(format_source(&output).expect("idempotence"), output);
}

#[test]
fn suspends_is_preserved_when_reordering_legal_declaration_modifiers() {
    let source = "suspends pub fnc run() -> int {\n  return 2;\n}\n";
    let output = format_source(source).expect("format");
    assert!(
        output.contains("pub suspends fnc run() -> int {"),
        "{output}"
    );
    assert_eq!(format_source(&output).expect("idempotence"), output);
}

#[test]
fn invalid_async_suspends_pair_is_not_normalized_to_drop_either_effect() {
    let source = "async suspends fnc bad() -> int {\n  return 1;\n}\n";
    let output = format_source(source).expect("format");
    assert!(output.contains("async suspends fnc bad()"), "{output}");
    assert!(!output.contains("async fnc bad()"), "{output}");
    assert!(!output.contains("suspends fnc bad()") || output.contains("async suspends fnc bad()"));
}

#[test]
fn task_and_suspends_lambda_are_preserved_as_distinct_effects() {
    let source = "fnc launch() -> void {\n  let Task<int> work = nb task {\n    return (suspends |int x| -> x)(7);\n  };\n  return;\n}\n";
    let output = format_source(source).expect("format");
    assert!(output.contains("nb task {"), "{output}");
    assert!(output.contains("suspends |int x| -> x"), "{output}");
    assert_eq!(format_source(&output).expect("idempotence"), output);
}
