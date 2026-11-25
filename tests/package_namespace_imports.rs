use oreslang_format::format_source;

#[test]
fn package_shorthand_and_explicit_alias_remain_canonical_and_idempotent() {
    let source = "import pkg 'z';\nimport pkg 'core.telemetry' as t;\nimport module math from pkg 'utils';\nimport * as helpers from './helpers.atto';\n";
    let once = format_source(source).unwrap();
    for form in [
        "import pkg 'z';",
        "import pkg 'core.telemetry' as t;",
        "import module math from pkg 'utils';",
        "import * as helpers from './helpers.atto';"
    ] {
        assert!(once.contains(form), "{form} changed: {once}");
    }
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn pure_unused_package_import_is_disabled_only_when_binding_is_unreferenced() {
    let unreferenced = "import pkg 'z'; // oresfmt: pure-import\npub fnc main(): void { return; }\n";
    let out = format_source(unreferenced).unwrap();
    assert!(out.contains("// import pkg 'z'; // oresfmt: pure-import"), "{out}");

    let referenced = "import pkg 'z'; // oresfmt: pure-import\npub fnc main(): void { z.math.square(7); return; }\n";
    let out = format_source(referenced).unwrap();
    assert!(out.contains("import pkg 'z'; // oresfmt: pure-import"), "{out}");
    assert!(!out.contains("// import pkg 'z';"), "{out}");
}

#[test]
fn package_alias_is_used_as_binding_for_pure_unused_analysis() {
    let source = "import pkg '@scope/math' as math; // oresfmt: pure-import\npub fnc main(): void { math.square(7); return; }\n";
    let out = format_source(source).unwrap();
    assert!(out.contains("import pkg '@scope/math' as math;"), "{out}");
    assert!(!out.contains("// import pkg '@scope/math'"), "{out}");
}

#[test]
fn invalid_or_unsupported_package_names_are_never_silently_removed() {
    for import in [
        "import pkg 'my-lib';",
        "import pkg 'core..math';",
        "import pkg '';",
        "import pkg z;",
        "import pkg '+/math';"
    ] {
        let source = format!("{import} // oresfmt: pure-import\npub fnc main(): void {{ return; }}\n");
        let out = format_source(&source).unwrap();
        assert!(!out.contains(&format!("// {import}")), "{out}");
    }
}

#[test]
fn mixed_static_imports_keep_original_order() {
    let source = "import pkg 'z';\nimport * as x from './relative-file.atto';\nimport pkg 'utils' as u;\n";
    let out = format_source(source).unwrap();
    let a = out.find("import pkg 'z';").unwrap();
    let b = out.find("import * as x from './relative-file.atto';").unwrap();
    let c = out.find("import pkg 'utils' as u;").unwrap();
    assert!(a < b && b < c, "{out}");
}
