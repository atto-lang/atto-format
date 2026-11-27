use oreslang_format::format_source;

// Lexical preservation tests only. Compiler roundtrip and guest behavior
// belong in atto-platform-conformance; no declaration rewrite is justified
// until all required compiler lanes agree.
#[test]
fn local_binding_forms_keep_qualifiers_and_cast_position() {
    let forms = [
        "int x = 5;",
        "string name = \"Alice\";",
        "const int y = 10;",
        "const y : int = 10;",
        "const y :int = 10;",
        "const y = 10 as int;",
        "const y as int = 10;",
        "val List[int] numbers = [1, 2, 3];",
        "const mut List[int] other = [4, 5, 6];",
        "let int z = 10;",
        "let z :int = 10;",
        "let z as int = 10;",
    ];
    for form in forms {
        let source = format!("fnc main(): int {{\n  {form}\n  return 42;\n}}\n");
        let formatted = format_source(&source).expect(form);
        // Every test input is one ordinary declaration, with no whitespace
        // inside its string literal. Ignore layout only, not token spelling.
        let compact: String = form.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(formatted.lines().any(|line| {
            line.chars().filter(|c| !c.is_whitespace()).collect::<String>() == compact
        }), "declaration changed: {form} => {formatted}");
        assert_eq!(format_source(&formatted).unwrap(), formatted, "{form}");
    }
}

#[test]
fn shadowing_and_cast_forms_remain_in_source_order() {
    let source = "fnc main(): int {\n  let int z = 10;\n  let z :int = 20;\n  let z as int = 30;\n  let z = 42 as int;\n  return z;\n}\n";
    let formatted = format_source(source).unwrap();
    let compact: String = formatted.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(compact.contains("letintz=10;letz:int=20;letzasint=30;letz=42asint;"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
