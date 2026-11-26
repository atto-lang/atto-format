use oreslang_format::format_source;

#[test]
fn nominal_method_prefix_is_optional_and_fnc_is_removed() {
    let source = r#"define interface Shape as
  fnc area(): int;
  mtd size(): int;
end
define trait Named as
  fnc name(): string;
  mtd label(): string;
end
define struct Value as
  pub fnc get(): int { return 42; }
  pub mtd other(): int { return 7; }
end
define class C as
  pub fnc get(): int { return 42; }
  pub mtd other(): int { return 7; }
  pub static fnc make(): int { return 42; }
end
define contract Api as
  fnc run(): int;
end
define static contract Factory as
  fnc make(): int;
end
define module M as
  pub fnc run(): int { return 42; }
end
pub mtd (C self) outside(): int { return 42; }
pub fnc main(): int { return 42; }
"#;
    let got = format_source(source).unwrap();
    for name in ["area", "name", "get"] {
        assert!(!got.contains(&format!("fnc {name}(")), "{got}");
    }
    for expected in ["area() => int;", "name() => string;", "mtd size()", "mtd label()", "mtd other()", "static fnc make(", "fnc run() => int;", "fnc make() => int;", "pub fnc run(", "pub mtd (C self) outside(", "pub fnc main("] {
        assert!(got.contains(expected), "missing {expected}: {got}");
    }
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn braced_method_signatures_keep_comments_and_explicit_mtd() {
    let input = r#"define interface Shape {
  // fnc unchanged(): int;
  fnc area(): int;
  mtd size(): int;
}
"#;
    let got = format_source(input).unwrap();
    assert!(got.contains("// fnc unchanged(): int;"), "{got}");
    assert!(got.contains("area() => int;"), "{got}");
    assert!(!got.contains("fnc area("), "{got}");
    assert!(got.contains("mtd size()"), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}
