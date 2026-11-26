use oreslang_format::format_source;

// An omitted visibility marker is export-eligible; written private is not.
// Formatting must not turn an invalid export into an admitted public binding.
#[test]
fn preserves_explicit_private_export_provenance() {
    for (header, canonical) in [
        ("define private class Foo as", "define private class Foo as"),
        ("private define class Foo as", "define private class Foo as"),
    ] {
        for directive in ["export pub Foo;", "export pub[pkg] class Foo;"] {
            for forward in [true, false] {
                let decl = format!("{header}\nend\n");
                let src = if forward { format!("{directive}\n{decl}") }
                    else { format!("{decl}{directive}\n") };
                let got = format_source(&src).expect("format");
                assert!(got.contains(canonical), "lost explicit private: {got}");
                assert!(got.contains(directive), "changed export scope: {got}");
                assert_eq!(format_source(&got).expect("second format"), got);
            }
        }
    }
}

#[test]
fn preserves_internal_and_external_linkage_markers() {
    let src = "internal fnc helper(): int { return 42; }\nexternal fnc host_clock_ns(): int;\n";
    let got = format_source(src).expect("format");
    assert!(got.contains("internal fnc helper("), "{got}");
    assert!(got.contains("external fnc host_clock_ns("), "{got}");
    assert!(!got.contains("pub fnc"), "{got}");
    assert_eq!(format_source(&got).expect("second format"), got);
}
