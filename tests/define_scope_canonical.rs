use oreslang_format::{format_source, is_formatted};

#[test]
fn canonical_top_level_define_pub_and_nested_member_omits_define() {
    let input = r#"pub define class Cart as
  pub define fnc new() -> Cart {
    return new Cart();
  }
  define pub fnc checkout() -> void {
    return;
  }
end

define module Helpers as
  define pub fnc ping() -> int {
    return 1;
  }
end

pub define fnc entry() -> int {
  return 7;
}
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("define pub class Cart as"), "{formatted}");
    assert!(formatted.contains("  pub fnc new() -> Cart {"), "{formatted}");
    assert!(!formatted.contains("pub define fnc"), "{formatted}");
    assert!(!formatted.contains("define pub fnc checkout"), "{formatted}");
    assert!(formatted.contains("define module Helpers as"), "{formatted}");
    assert!(formatted.contains("  pub fnc ping() -> int {"), "{formatted}");
    assert!(formatted.contains("define pub fnc entry() -> int {"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn declaration_scope_is_not_expression_scope() {
    let input = r#"define pub class Example as
  pub define fnc new() -> Example {
    val text = "pub define fnc fake()";
    // pub define fnc fake()
    return new Example();
  }
end
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("  pub fnc new() -> Example {"), "{formatted}");
    assert!(formatted.contains("\"pub define fnc fake()\""), "{formatted}");
    assert!(formatted.contains("// pub define fnc fake()"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn commented_or_invalid_prefixes_are_not_reinterpreted() {
    let input = r#"define class Example as
  pub /* keep */ define fnc new() -> Example {
    return new Example();
  }
end
"#;
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("pub /* keep */ define fnc new()"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
