use oreslang_format::{format_source, is_formatted};

#[test]
fn canonicalizes_exact_symbol_self_identity_only() {
    let source = "export symbol #ready as #ready;\nfnc keep(): void { return; }\n";
    let formatted = format_source(source).expect("self-identity export should format");
    assert!(formatted.contains("export symbol #ready;"), "{formatted}");
    assert!(!formatted.contains("export symbol #ready as #ready;"), "{formatted}");
    assert!(formatted.contains("fnc keep() -> void"), "{formatted}");
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn canonicalizes_scoped_symbol_self_identity_without_changing_scope_spelling() {
    for head in [
        "export[file]",
        "export[pkg]",
        "export[package]",
        "export(file)",
        "export(pkg)",
        "export(package)",
    ] {
        let source = format!("{head} symbol #ready as #ready;\nfnc keep(): void {{ return; }}\n");
        let formatted = format_source(&source).unwrap();
        assert!(
            formatted.contains(&format!("{head} symbol #ready;")),
            "{formatted}"
        );
        assert!(!formatted.contains("as #ready"));
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}

#[test]
fn invalid_or_semantically_distinct_export_forms_are_never_repaired() {
    for source in [
        "export symbol #ready as #other;\n",
        "export symbol #ready as Ready;\n",
        "export symbol ready as #ready;\n",
        "export const ready as 'ready';\n",
        "export string ready as 'ready';\n",
        "export class Foo as \"foo\";\n",
        "export *;\n",
        "export dep.X;\n",
        "export actor {};\n",
        "export actor {A,,B};\n",
    ] {
        let formatted = format_source(source).expect("formatter must preserve compiler-owned invalid syntax");
        assert!(
            formatted.contains(source.trim()),
            "formatter changed invalid export semantics: {source:?} -> {formatted:?}"
        );
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}

#[test]
fn export_order_grouping_and_aliases_are_preserved() {
    let source = "export class B as Bee;\nexport class {A, C as Sea,};\nimport class Dep from './dep';\ndefine class A as\nend\ndefine class B as\nend\ndefine class C as\nend\n";
    let formatted = format_source(source).unwrap();

    let first = formatted.find("export class B as Bee;").unwrap();
    let second = formatted.find("export class {A, C as Sea,};").unwrap();
    let import = formatted.find("import class Dep from './dep';").unwrap();

    assert!(first < second && second < import, "{formatted}");
    assert!(formatted.contains("export class {A, C as Sea,};"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn strings_and_comments_containing_export_text_are_opaque() {
    let source = r#"fnc text(): string {
return "export symbol #ready as #ready;";
}
// export symbol #ready as #ready;
/* export symbol #ready as #ready; */
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("\"export symbol #ready as #ready;\""), "{formatted}");
    assert!(formatted.contains("// export symbol #ready as #ready;"), "{formatted}");
    assert!(formatted.contains("/* export symbol #ready as #ready; */"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
