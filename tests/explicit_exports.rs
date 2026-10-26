use oreslang_format::format_source;

#[test]
fn canonicalizes_export_scope_aliases_without_reordering_items() {
    let input = "export(package) class {X, Y as Z,};\nexport(file) type Internal;\n";
    let out = format_source(input).unwrap();
    assert!(out.contains("export[pkg] class {X, Y as Z,};"));
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
fn export_formatting_is_idempotent() {
    let once = format_source(
        "export(package) class {X, Y as Z,};\nexport symbol #ready as #ready;\n",
    )
    .unwrap();
    let twice = format_source(&once).unwrap();
    assert_eq!(once, twice);
}
