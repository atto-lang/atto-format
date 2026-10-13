use oreslang_format::format_source;

#[test]
fn converts_legacy_class_fields_without_touching_locals() {
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
  pub int foo: 5;
  final int bar: 6;
  final Map m: new Map();
  final mut Map m1: new Map();
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
  final mut Map<string, int> fixed: new Map();
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
    assert!(formatted.contains("get(): int;"));
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
    assert!(formatted.contains("final(): int"));
    assert!(formatted.contains("name: string = \"value\";"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn rejects_duplicate_mut_modifier_by_preserving_source_for_compiler() {
    let input = r#"define class Foo as
  val mut Map m = new Map();
end
"#;
    assert!(format_source(input).unwrap().contains("val mut Map m = new Map();"));
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
  final Box<messages=Message> queue: new Box();
  pub Map<string, Box<messages=Message>> routes: new Map();
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
    assert!(formatted.contains("calculate(int x): int {"));
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
    assert!(formatted.contains("val int value;"));
    assert!(formatted.contains("let int accumulator;"));
    assert!(formatted.contains("const int obsolete;"));
    assert!(!formatted.contains("final mut int value;"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
