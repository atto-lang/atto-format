use oreslang_format::format_source;

#[test]
fn preserves_explicit_bindings_and_elides_only_redundant_val() {
    let input = r#"define class Foo as
  pub let int foo = 5;
  const int bar = 6;
  const Map m = new Map();
  val Map m1 = new Map();
  final mut Map already: new Map();
  constructor() {
    let int local = 2;
    const int other = 3;
  }
end
"#;
    let expected = r#"define class Foo as
  pub let int foo: 5;
  const int bar: 6;
  const Map m: new Map();
  Map m1: new Map();
  final mut Map already: new Map();
  constructor() {
    let int local = 2;
    const int other = 3;
  }
end
"#;
    assert_eq!(format_source(input).unwrap(), expected);
    assert_eq!(format_source(expected).unwrap(), expected);
}

#[test]
fn canonicalizes_bare_typed_fields_and_generic_types() {
    let input = r#"define class Foo as
  int x = 1;
  mut Map<string, int> values = new Map();
  const mut Map<string, int> fixed = new Map();
end
"#;
    let expected = r#"define class Foo as
  int x: 1;
  mut Map<string, int> values: new Map();
  const mut Map<string, int> fixed: new Map();
end
"#;
    assert_eq!(format_source(input).unwrap(), expected);
    assert_eq!(format_source(expected).unwrap(), expected);
}

#[test]
fn preserves_actor_state_and_trait_signatures() {
    let input = r#"define actor Worker as
  const int value = 5;
end

define trait T as
  get(): int;
end
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("const int value = 5;"));
    assert!(formatted.contains("get() => int;"));
}

#[test]
fn leaves_suspicious_fields_and_methods_alone() {
    let input = r#"define class Foo as
  pub fnc calculate(int value): int {
    return value;
  }
  final(): int { return 2; }
  name: string = "value";
end
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("final() -> int"));
    assert!(formatted.contains("name: string = \"value\";"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn rejects_duplicate_mut_modifier_by_preserving_source_for_compiler() {
    let input = r#"define class Foo as
  val mut Map m = new Map();
end
"#;
    assert!(
        format_source(input)
            .unwrap()
            .contains("val mut Map m = new Map();")
    );
}

#[test]
fn generic_named_type_arguments_do_not_become_initializers() {
    let input = r#"define class Pipeline as
  const Box<messages=Message> queue = new Box();
  pub let Map<string, Box<messages=Message>> routes = new Map();
  final int answer: 42;
end
"#;
    let expected = r#"define class Pipeline as
  const Box<messages=Message> queue: new Box();
  pub let Map<string, Box<messages=Message>> routes: new Map();
  final int answer: 42;
end
"#;
    assert_eq!(format_source(input).unwrap(), expected);
    assert_eq!(format_source(expected).unwrap(), expected);
}

#[test]
fn generic_nested_colons_and_method_signatures_are_not_rewritten() {
    let source = r#"define class Foo as
  final Map<value: string> fields = build();
  calculate(int x): int {
    return x;
  }
  final mut Map<int, string> cache: new Map();
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("final Map<value: string> fields: build();"));
    assert!(formatted.contains("calculate(int x) -> int {"));
    assert!(formatted.contains("final mut Map<int, string> cache: new Map();"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn comment_separated_prefixes_are_not_silently_normalized() {
    let source = "define class Foo as\n  const /* ownership */ int value = 1;\nend\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("const /* ownership */ int value = 1;"));
}

#[test]
fn named_struct_declarations_are_left_unchanged_until_compiler_parity() {
    let source = r#"define struct Pair as
  const int left = 1;
  val int right = 2;
end
"#;
    assert_eq!(format_source(source).unwrap(), source);
}

#[test]
fn uninitialized_legacy_constructor_fields_are_not_rewritten_into_invalid_final_fields() {
    let source = r#"define class Box as
  val int value;
  let int accumulator;
  const int obsolete;
  constructor(int value) {
    self.value = value;
  }
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("int value;"));
    assert!(formatted.contains("let int accumulator;"));
    assert!(formatted.contains("const int obsolete;"));
    assert!(!formatted.contains("final mut int value;"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn implicit_val_only_changes_direct_class_members() {
    let input = r#"define class Envelope as
  pub val String method;
  val Map<string, int> values: new Map();
  let int changes: 0;
  const int immutable: 3;
  final int once;
  initialize(int value) -> void ::
    self.once = value;
    val int local = value;
  end
end

define actor Receiver as
  val int state: 0;
end
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("pub String method;"));
    assert!(formatted.contains("Map<string, int> values: new Map();"));
    assert!(formatted.contains("let int changes: 0;"));
    assert!(formatted.contains("const int immutable: 3;"));
    assert!(formatted.contains("final int once;"));
    assert!(formatted.contains("val int local = value;"));
    assert!(formatted.contains("val int state: 0;"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn typed_initialize_end_headers_are_canonical_and_idempotent() {
    for header in [
        "initialize(int id): void",
        "initialize(int id) -> void",
        "initialize(int id): void ::",
        "initialize(int id) -> void ::",
    ] {
        let source = format!(
            "define class Connection as\n  final int connection_id;\n  {header}\n    self.connection_id = id;\n  end\nend\n"
        );
        let formatted = format_source(&source).unwrap();
        assert!(formatted.contains("initialize(int id) -> void ::"), "{formatted}");
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}

#[test]
fn rejects_callable_as_without_touching_type_header_as() {
    assert!(format_source(
        "define class Connection as\n  initialize(int id) -> void as\n    self.id = id;\n  end\nend\n"
    ).is_err());
    assert!(format_source("fnc f() -> int as return 1; end\n").is_err());
    assert!(format_source("define class Connection as\nend\n").is_ok());
}

#[test]
fn leaves_braced_initializer_body_intact_until_ast_backed_conversion() {
    let source = "define class C as\n  initialize(int id): void {\n    return;\n  }\nend\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("initialize(int id) -> void {"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn compact_callable_as_in_body_is_not_mistaken_for_a_header_separator() {
    let source = "fnc handle() -> void :: if maybe as const value then return; fi end\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("as const value"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn compact_callable_header_still_rejects_as_separator() {
    assert!(format_source("fnc handle() -> void as return; end\n").is_err());
}

#[test]
fn malformed_class_field_modifiers_are_preserved_for_compiler_diagnostics() {
    // A style-only tool must never erase the first modifier of an invalid
    // declaration and thereby turn a compiler error into a valid field.
    for declaration in [
        "val val int duplicate = 1;",
        "pub val val int duplicate = 1;",
        "val let int conflict = 1;",
        "let let int duplicate = 1;",
        "const final int conflict = 1;",
        "mut mut int duplicate = 1;",
        "pub private int conflicting_visibility = 1;",
    ] {
        let source = format!("define class Prefixes as\n  {declaration}\nend\n");
        let formatted = format_source(&source).unwrap();
        assert!(
            formatted.contains(declaration),
            "invalid declaration was changed: {declaration:?} -> {formatted:?}"
        );
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}
