use oreslang_format::format_source;

#[test]
fn canonicalizes_public_declared_type_only_header_order() {
    let source = "pub declare type AccountId = int;\n"
        .to_string()
        + "pub declare trait Tagged as\n"
        + "fnc tag(): int;\nend\n"
        + "pub declare interface Endpoint as\n"
        + "fnc request(): int;\nend\n";
    let formatted = format_source(&source).expect("type-only declarations must format");
    assert!(formatted.contains("declare pub type AccountId = int;"), "{formatted}");
    assert!(formatted.contains("declare pub trait Tagged as"), "{formatted}");
    assert!(formatted.contains("declare pub interface Endpoint as"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn closes_nested_declared_trait_and_interface_frames() {
    let source = "define module api as\n"
        .to_string()
        + "declare pub trait Tagged as\n"
        + "fnc tag(): int;\n"
        + "end\n"
        + "declare pub interface Endpoint as\n"
        + "fnc request(): int;\n"
        + "end\n"
        + "end\n";
    let formatted = format_source(&source).expect("nested frames must close");
    assert!(formatted.contains("  declare pub trait Tagged as"), "{formatted}");
    assert!(formatted.contains("  declare pub interface Endpoint as"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn declare_remains_contextual_and_literals_are_unmodified() {
    let source = "fnc declare(int x): int { return x; }\n"
        .to_string()
        + "fnc describe(): string { return \"pub declare trait Foo\"; }\n";
    let formatted = format_source(&source).unwrap();
    assert!(formatted.contains("fnc declare(int x)"), "{formatted}");
    assert!(formatted.contains("\"pub declare trait Foo\""), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn legacy_define_interface_and_trait_remain_supported() {
    let source = "define interface Api as\n"
        .to_string()
        + "fnc get(): int;\nend\n"
        + "define trait Tagged as\n"
        + "fnc tag(): int;\nend\n";
    let formatted = format_source(&source).expect("existing declarations remain valid");
    assert!(formatted.contains("define interface Api"), "{formatted}");
    assert!(formatted.contains("define trait Tagged"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
