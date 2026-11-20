use oreslang_format::{format_source, is_formatted};

#[test]
fn direct_instance_fnc_uses_current_method_contract_without_touching_static_or_module() {
    let input = "define class Counter as\n  pub fnc inc(int value) => int {\n    return value;\n  }\n  static fnc count() => int {\n    return 1;\n  }\nend\n\nfnc global(int x) => int {\n  return x;\n}\n";
    let result = format_source(input).unwrap();
    assert!(result.contains("pub inc("), "{result}");
    assert!(!result.contains("pub fnc inc("), "{result}");
    assert!(result.contains("static fnc count("), "{result}");
    assert!(result.contains("fnc global("), "{result}");
    assert!(is_formatted(&result).unwrap());
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn method_spelling_inside_quoted_string_is_not_rewritten() {
    let input = "define class Counter as\n  pub fnc method_name() => String {\n    return \"fnc fake() -> void\";\n  }\nend\n";
    let result = format_source(input).unwrap();
    assert!(result.contains("\"fnc fake() -> void\""), "{result}");
    assert!(result.contains("pub method_name("), "{result}");
}
