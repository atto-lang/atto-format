use oreslang_format::{format_source, is_formatted};

fn canonical(input: &str, expected: &str) {
    let formatted = format_source(input).unwrap();
    assert_eq!(formatted, expected, "input: {input:?}");
    assert_eq!(format_source(&formatted).unwrap(), formatted, "idempotence");
    assert!(is_formatted(&formatted).unwrap(), "canonical text should pass --check");
    if input != expected {
        assert!(!is_formatted(input).unwrap(), "private spelling should not pass --check");
    }
}

#[test]
fn strips_private_from_named_declarations_but_keeps_define_and_pub() {
    for (input, expected) in [
        ("define private class Box as\nend\n", "define class Box as\nend\n"),
        ("private define class Box as\nend\n", "define class Box as\nend\n"),
        ("private class Box as\nend\n", "class Box as\nend\n"),
        ("define private trait T as\nend\n", "define trait T as\nend\n"),
        ("private trait T as\nend\n", "trait T as\nend\n"),
        ("define private struct S as\nend\n", "define struct S as\nend\n"),
        ("private struct S as\nend\n", "struct S as\nend\n"),
        ("define private interface I as\nend\n", "define interface I as\nend\n"),
        ("define private contract C as\nend\n", "define contract C as\nend\n"),
        ("define private module Service as\nend\n", "define module Service as\nend\n"),
        ("private define module Service as\nend\n", "define module Service as\nend\n"),
        ("pub define module Service as\nend\n", "define pub module Service as\nend\n"),
        ("define pub module Service as\nend\n", "define pub module Service as\nend\n"),
        ("define private type Alias = int;\n", "define type Alias = int;\n"),
        ("private type Alias = int;\n", "type Alias = int;\n"),
        ("pub define class Box as\nend\n", "define pub class Box as\nend\n"),
        ("define pub class Box as\nend\n", "define pub class Box as\nend\n"),
        ("define private actor Worker as\nend\n", "define actor Worker as\nend\n"),
        ("private define actor Worker as\nend\n", "define actor Worker as\nend\n"),
        ("define pub actor Worker as\nend\n", "define pub actor Worker as\nend\n"),
        ("pub type Exported = int;\n", "pub type Exported = int;\n"),
    ] {
        canonical(input, expected);
    }
}

#[test]
fn strips_private_from_functions_and_actor_callables() {
    for (input, expected) in [
        ("private fnc work(): void {\n}\n", "fnc work() -> void {\n}\n"),
        ("private static fnc create(): int {\n}\n", "static fnc create() -> int {\n}\n"),
        ("private async fnc load(): void {\n}\n", "async fnc load() -> void {\n}\n"),
        ("private routine stop(): void {\n}\n", "routine stop() -> void {\n}\n"),
        ("private actor fnc send(): void {\n}\n", "actor fnc send() -> void {\n}\n"),
        ("private untrusted actor fnc receive(): void {\n}\n",
         "untrusted actor fnc receive() -> void {\n}\n"),
    ] {
        canonical(input, expected);
    }
}

#[test]
fn strips_private_from_class_fields_methods_and_constructor() {
    let source = r#"define pub class Vault as
private final int secret: 42
private val String label: "a"
private let int counter: 0
private constructor() {
}
private get_secret(): int {
return self.secret;
}
private method explicit(): int {
return 7;
}
private fnc legacy(): int {
return 8;
}
async private modified(): int {
return 9;
}
private abstract required() => int;
private [Symbol.iterator](): Array<int> {
return arr[1, 2];
}
pub get_label(): String {
return self.label;
}
end
"#;
    let formatted = format_source(source).unwrap();
    for part in [
        "  final int secret: 42",
        "  val String label: \"a\"",
        "  let int counter: 0",
        "  constructor() {",
        "  get_secret() -> int {",
        "  method explicit() -> int {",
        "  legacy() -> int {",
        "  async modified() -> int {",
        "  abstract required() => int;",
        "  [Symbol.iterator]() -> Array<int> {",
        "  pub get_label() -> String {",
    ] {
        assert!(formatted.contains(part), "missing {part:?} in {formatted}");
    }
    assert!(!formatted.contains("private "), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn strips_private_from_module_bindings_without_rewriting_expressions() {
    let source = r#"define module Library as
private val int secret = 1;
private fnc hidden(): int {
return 1;
}
pub val int exposed = 2;
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("  val int secret = 1;"), "{formatted}");
    assert!(formatted.contains("  fnc hidden() -> int {"), "{formatted}");
    assert!(formatted.contains("  pub val int exposed = 2;"), "{formatted}");
    assert!(!formatted.contains("private "));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn conflicting_or_obscured_prefixes_stay_for_compiler_errors() {
    for input in [
        "pub private fnc contradictory() {\n}\n",
        "private pub val int invalid = 1;\n",
        "private private fnc duplicate() {\n}\n",
        "define pub private class Conflict as\nend\n",
        "private /* keep */ fnc annotated() {\n}\n",
        "private /* keep */ val int secret = 1;\n",
        "// private fnc example()\n",
        "val String text = \"private fnc example()\";\n",
    ] {
        let once = format_source(input).unwrap();
        assert!(once.contains("private"), "rewrote invalid/opaque input: {input} -> {once}");
        assert_eq!(format_source(&once).unwrap(), once);
    }
}

#[test]
fn private_type_first_class_fields_are_canonicalized() {
    let source = r#"define class Typed as
private int count: 5
private String name = "A";
private Map<String, int> counts: values
private Array<int> items = values;
private id: int
end
"#;
    let got = format_source(source).unwrap();
    for expected in [
        "  int count: 5",
        "  String name = \"A\";",
        "  Map<String, int> counts: values",
        "  Array<int> items = values;",
        "  id: int",
    ] {
        assert!(got.contains(expected), "missing {expected:?} in {got}");
    }
    assert!(!got.contains("private "), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn type_first_field_modifier_conflicts_must_not_be_legalized() {
    // Redundant-private normalization must NEVER turn invalid source into
    // an exported field by removing the first visibility token.
    for declaration in [
        "private pub int secret: 42",
        "private private int secret: 42",
        "private pub Map<String, int> secret: values",
        "private private Array<int> secrets = values;",
    ] {
        let source = format!("define class Vault as\n{declaration}\nend\n");
        let formatted = format_source(&source).unwrap();
        assert!(formatted.contains(declaration),
            "visibility conflict erased: {source:?} -> {formatted:?}");
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}

#[test]
fn invalid_nested_visibility_stays_visible_to_compiler() {
    // These constructs are invalid declarations in local control-flow
    // scopes. Formatting must not silently make them valid.
    let source = r#"pub fnc outer(): void {
private define module Hidden as
end
private fnc nested(): void {
}
private class Nested as
end
return;
}
"#;
    let got = format_source(source).unwrap();
    for preserved in [
        "private define module Hidden as",
        "private fnc nested()",
        "private class Nested as",
    ] {
        assert!(got.contains(preserved), "compiler diagnostic erased: {preserved:?} in {got}");
    }
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn invalid_nested_type_and_module_declarations_inside_class_stay_invalid() {
    let input = r#"define class Outer as
private define module Inner as
end
private class Nested as
end
end
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("private define module Inner as"), "{formatted}");
    assert!(formatted.contains("private class Nested as"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn module_modifier_conflicts_and_comments_are_not_rewritten() {
    for input in [
        "define pub private module Service as\nend\n",
        "private pub define module Service as\nend\n",
        "define private private module Service as\nend\n",
        "private /* keep */ define module Service as\nend\n",
        "private define /* keep */ module Service as\nend\n",
        "// private define module Service as\n",
        "val String text = \"private define module Service as\";\n",
    ] {
        let got = format_source(input).unwrap();
        assert!(got.contains("private"), "unexpected rewrite: {input:?} -> {got}");
        assert_eq!(format_source(&got).unwrap(), got);
    }
}

#[test]
fn preserves_comments_when_removing_standalone_modifier() {
    canonical(
        "private fnc foo(): void { // keep\n}\n",
        "fnc foo() -> void { // keep\n}\n",
    );
}

#[test]
fn does_not_strip_private_from_invalid_local_bindings() {
    let source = r#"pub fnc example(): void {
private val int invalid = 3;
if true then
private let int also_invalid = 4;
fi
}
"#;
    let got = format_source(source).unwrap();
    assert!(got.contains("private val int invalid = 3;"), "{got}");
    assert!(got.contains("private let int also_invalid = 4;"), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn private_after_other_instance_method_modifiers_is_stripped() {
    let source = r#"define class Example as
async private load(): int {
return 1;
}
private async flush(): int {
return 2;
}
abstract private required() => int;
end
"#;
    let got = format_source(source).unwrap();
    assert!(got.contains("  async load() -> int {"), "{got}");
    assert!(got.contains("  async flush() -> int {"), "{got}");
    assert!(got.contains("  abstract required() => int;"), "{got}");
    assert!(!got.contains("private "), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn malformed_module_prefixes_are_never_rewritten_into_exports() {
    // Unknown/misordered modifiers are compiler errors, not syntax sugar.
    // A formatter must not turn them into successful public or private code.
    for source in [
        "define module pub Internal as\nend\n",
        "define module private Internal as\nend\n",
        "define private module pub Internal as\nend\n",
        "define pub module private Internal as\nend\n",
        "module define pub Internal as\nend\n",
        "module pub define Internal as\nend\n",
        "private module Internal as\nend\n",
        "pub module Internal as\nend\n",
        "define module define Internal as\nend\n",
    ] {
        let formatted = format_source(source).expect("formatter must keep invalid syntax diagnosable");
        let first_line = source.lines().next().unwrap();
        assert!(
            formatted.lines().any(|line| line == first_line),
            "rewrote invalid declaration into different semantics: {source:?} -> {formatted:?}"
        );
        assert_eq!(
            format_source(&formatted).unwrap(),
            formatted,
            "normalization must be idempotent for {source:?}"
        );
    }
}

#[test]
fn both_explicit_private_module_orders_match_the_implicit_default() {
    let source = "define module Internal as\n  pub fnc visible(): int {\n    return 1;\n  }\nend\n";
    let baseline = format_source(source).unwrap();
    for header in [
        "private define module Internal as",
        "define private module Internal as",
    ] {
        let variant = source.replacen("define module Internal as", header, 1);
        let formatted = format_source(&variant).unwrap();
        assert_eq!(baseline, formatted, "{header} should be semantically identical");
        assert_eq!(formatted, format_source(&formatted).unwrap());
    }
    let public = source.replacen("define module Internal as", "pub define module Internal as", 1);
    assert!(format_source(&public).unwrap().starts_with("define pub module Internal as\n"));
}

#[test]
fn supported_legacy_type_and_class_modifier_orders_stay_canonical() {
    for (input, expected) in [
        ("define class pub Vault as\nend\n", "define pub class Vault as\nend\n"),
        ("class define pub Vault as\nend\n", "define pub class Vault as\nend\n"),
        ("define interface pub Api as\nend\n", "define pub interface Api as\nend\n"),
        ("define contract private Api as\nend\n", "define contract Api as\nend\n"),
        ("define type pub Alias = int;\n", "define pub type Alias = int;\n"),
        ("define type private Alias = int;\n", "define type Alias = int;\n"),
    ] {
        let formatted = format_source(input).unwrap();
        assert_eq!(formatted, expected, "legacy grammar regression for {input:?}");
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}
