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

#[test]
fn end_delimited_root_define_callable_closes_without_phantom_frame() {
    for source in [
        "define pub fnc answer() -> int ::\n  return 42;\nend\n",
        "pub define fnc answer() -> int ::\n  return 42;\nend\n",
    ] {
        let formatted = format_source(source).expect("end-delimited declaration must format");
        assert!(formatted.starts_with("define pub fnc answer() -> int ::"), "{formatted}");
        assert!(formatted.trim_end().ends_with("end"), "{formatted}");
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}

#[test]
fn malformed_nested_type_is_not_repaired_into_plausible_source() {
    let input = "define class Outer as\n  define class Inner as\n  end\nend\n";
    // Formatting invalid guest syntax must not silently manufacture a
    // legitimate declaration shape inside a class. The compiler rejects it.
    if let Ok(formatted) = format_source(input) {
        assert!(formatted.contains("define class Inner as"), "{formatted}");
    }
}

#[test]
fn nested_define_removal_preserves_modifier_conflicts_and_literal_payloads() {
    let source = "define module Example as\n  pub define fnc run() -> string {\n    return \"define pub fnc from_string()\";\n  }\n  define pub pub fnc bad() -> void {\n    return;\n  }\nend\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("return \"define pub fnc from_string()\";"), "{formatted}");
    // Do not silently collapse duplicate visibility into a valid declaration.
    assert!(formatted.contains("pub pub fnc bad()"), "{formatted}");
}
