//! Target capture punctuation and per-leaf qualifiers; not guest admission.
use oreslang_format::format_source;

fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn capture_patterns_keep_delimiters_and_binding_modes() {
    for decl in [
        "const (a, b, c) as r = foo();",
        "const {a, b, c} as r = foo();",
        "const [a, b, c] as r = foo();",
        "const [a, mut b, c] as r = foo();",
        "const [a, b, let c] as r = foo();",
        "[const a, let mut b, val c] as r = foo();",
        "const (a, b, c) as Record r = foo();",
        "const (a, b, c) as Array r = foo();",
        "const ({name, age} as person, [x, let mut y, z] as coords) as r = foo();",
        "const () as t = foo();",
        "const {} as r = foo();",
        "const [] as a = foo();",
        "const [a, let b, c] as let r = foo();",
    ] {
        let source = format!("fnc main() -> void {{\n{decl}\nreturn;\n}}\n");
        let formatted = format_source(&source).expect("capture source should be preserved");
        assert_eq!(compact(&formatted), compact(&source), "{decl}");
        assert_eq!(format_source(&formatted).unwrap(), formatted, "{decl}");
    }
}
