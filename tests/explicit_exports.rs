use oreslang_format::format_source;

#[test]
fn canonicalizes_export_scope_aliases_without_reordering_items() {
    let input = "export(package) class {X, Y as Z,};\nexport(file) type Internal;\n";
    let out = format_source(input).unwrap();
    // The existing brace formatter expands grouped exports across lines.
    // Preserve item order and punctuation while canonicalizing the scope.
    assert!(
        out.contains("export[pkg] class {\n  X, Y as Z,\n};"),
        "{out}"
    );
    assert!(out.contains("export[file] type Internal;"));
    assert!(out.find("X").unwrap() < out.find("Y as Z").unwrap());
}

#[test]
fn collapses_only_exact_symbol_self_identity() {
    assert_eq!(
        format_source("export symbol #ready as #ready;\n").unwrap(),
        "export symbol #ready;\n"
    );

    let mismatch = format_source("export symbol #ready as #other;\n").unwrap();
    assert_eq!(mismatch, "export symbol #ready as #other;\n");

    let binding_alias = format_source("export symbol #ready as Ready;\n").unwrap();
    assert_eq!(binding_alias, "export symbol #ready as Ready;\n");
}

#[test]
fn invalid_exports_are_not_repaired_into_different_semantics() {
    for source in [
        "export *;\n",
        "export from \"./dep\" {X};\n",
        "export string ready as 'ready';\n",
        "export const ready as 'ready';\n",
        "export dep.X;\n",
    ] {
        assert_eq!(format_source(source).unwrap(), source);
    }
}

#[test]
fn contextual_symbol_binding_exports_are_not_reinterpreted_as_symbol_identity_syntax() {
    for source in [
        "export symbol as PublicSymbol;\n",
        "export {symbol as PublicSymbol};\n",
        "export symbol;\n",
    ] {
        let out = format_source(source).unwrap();
        assert_eq!(out, source, "{source:?}");
        assert_eq!(format_source(&out).unwrap(), out);
    }
}

#[test]
fn export_formatting_is_idempotent() {
    let once =
        format_source("export(package) class {X, Y as Z,};\nexport symbol #ready as #ready;\n")
            .unwrap();
    let twice = format_source(&once).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn multiline_block_comment_export_text_is_opaque() {
    let source = "/*\nexport symbol #ready as #ready;\nexport(package) class X;\n*/\nfnc keep(): void { return; }\n";
    let out = format_source(source).unwrap();
    assert!(out.contains("export symbol #ready as #ready;"), "{out}");
    assert!(out.contains("export(package) class X;"), "{out}");
    assert!(!out.contains("/*\nexport symbol #ready;"), "{out}");
    assert_eq!(format_source(&out).unwrap(), out);
}
