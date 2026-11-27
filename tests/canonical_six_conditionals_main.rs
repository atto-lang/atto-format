use oreslang_format::format_source;

#[test]
fn six_conditional_spellings_are_preserved() {
    for (body, witnesses) in [
        ("return if true then 5 else 6 fi;", ["return if true then", "else", "fi"]),
        ("return true ? 5 : 6;", ["return true ?", ":", "6"]),
        ("return true ? { return 5; } : { return 6; };", ["return true ?", "return 5;", "return 6;"]),
        ("do if true then return 5; else return 6; fi", ["do if true then", "else", "fi"]),
        ("if true do return 5; else return 6; fi", ["if true do", "else", "fi"]),
        ("if true { return 5; } else { return 6; }", ["if true {", "} else {", "return 6;"]),
    ] {
        let source = format!("pub fnc main(): int {{\n{body}\n}}\n");
        let output = format_source(&source).unwrap_or_else(|e| panic!("{body}: {e:?}"));
        for witness in witnesses {
            assert!(output.contains(witness), "missing {witness} in {output}");
        }
        assert_eq!(format_source(&output).unwrap(), output, "{body}");
    }
}

#[test]
fn closed_braced_inner_if_does_not_steal_outer_fi() {
    let source = "pub fnc main(): int {\ndo if true then\nif false { return 7; }\nfi\nreturn 42;\n}\n";
    let output = format_source(source).unwrap();
    assert_eq!(output.matches("fi\n").count(), 1);
    assert!(output.contains("  fi\n  return 42;"));
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn legacy_explicit_brace_fi_preserved_and_missing_brace_rejected() {
    let source = "fnc main() {\nif true {\nwork();\n} fi\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("} fi"));
    assert_eq!(format_source(&output).unwrap(), output);
    assert!(format_source("fnc main() {\nif true {\nwork();\n}\n").is_err());
}
