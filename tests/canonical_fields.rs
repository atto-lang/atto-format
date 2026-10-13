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
