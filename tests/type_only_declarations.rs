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

#[test]
fn declare_and_define_complete_type_aliases_both_format_idempotently() {
    for source in [
        "declare type Code = int;\n",
        "define type Code = int;\n",
        "declare pub type Code = int;\n",
        "define pub type Code = int;\n",
    ] {
        let formatted = format_source(source).expect("completed type alias must format");
        assert!(formatted.contains("type Code = int;"), "{formatted}");
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}

#[test]
fn complete_type_aliases_preserve_module_end_balance() {
    let source = "define module api as\n".to_string()
        + "declare type Code = int;\n"
        + "fnc identity(Code value): Code { return value; }\n"
        + "define type OtherCode = int;\n"
        + "end\n";
    let formatted = format_source(&source).expect("completed aliases cannot open end blocks");
    assert!(formatted.contains("  declare type Code = int;"), "{formatted}");
    assert!(formatted.contains("  define type OtherCode = int;"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn named_alias_extension_method_preserves_contextual_mtd_and_on() {
    let source = "declare type Code = int;\n".to_string()
        + "define mtd double on Code(): int { return self + self; }\n"
        + "fnc calculate(Code code): int { return code.double(); }\n";
    let formatted = format_source(&source).expect("extension source must remain valid");
    assert!(formatted.contains("define mtd double on Code()"), "{formatted}");
    assert!(formatted.contains("code.double()"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}


#[test]
fn shared_complete_type_declaration_corpus_is_format_stable() {
    let contract = include_str!("../contracts/v1/type-declarations-conformance-v1.tsv");
    let mut ids = std::collections::HashSet::new();
    let mut count = 0;
    for line in contract
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<&str> = line.splitn(5, '\t').collect();
        assert_eq!(fields.len(), 5, "malformed shared vector: {line}");
        let (id, result) = (fields[0], fields[1]);
        assert!(ids.insert(id), "duplicate shared id: {id}");
        assert!(result == "+" || result == "-", "bad expected outcome: {id}");
        let source = fields[4].replace("\\n", "\n");
        match format_source(&source) {
            Ok(formatted) => {
                assert_eq!(
                    format_source(&formatted).expect("format-after-format"),
                    formatted,
                    "formatter non-idempotent: {id}"
                );
                // A formatter is not a compiler. An incomplete alias may format, but
                // must not be repaired into a misleading completed declaration.
                if id.starts_with("incomplete-") {
                    assert!(
                        !formatted.contains("Code ="),
                        "invalid alias repaired: {id}"
                    );
                }
                if result == "+" {
                    assert!(
                        formatted.contains("type ") || formatted.contains("trait "),
                        "missing type-only declaration in positive vector: {id}"
                    );
                }
            }
            Err(error) => {
                if result == "+" {
                    panic!("formatter rejected positive {id}: {error:?}");
                }
            }
        }
        count += 1;
    }
    assert_eq!(count, 22, "shared corpus drift");
}
