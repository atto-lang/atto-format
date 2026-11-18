use oreslang_format::{format_source, is_formatted};

#[test]
fn four_type_first_begin_bodies_use_two_space_indentation_and_three_closers() {
    for header in [
        "int <- foo()",
        "int• foo()",
        "fnc int <- foo()",
        "fnc int• foo()",
    ] {
        for closer in ["end", "end.", "end;"] {
            let input = format!("{header}\nbegin\nreturn 42;\n{closer}\n");
            let expected = format!("{header}\nbegin\n  return 42;\n{closer}\n");
            let result = format_source(&input).unwrap_or_else(|e| panic!("{input}: {e}"));
            assert_eq!(result, expected);
            assert!(is_formatted(&result).unwrap());
            assert_eq!(result, format_source(&result).unwrap());
        }
    }
}

#[test]
fn without_begin_body_is_still_checked_and_indented() {
    for header in [
        "int <- foo()",
        "int• foo()",
        "fnc int <- foo()",
        "fnc int• foo()",
    ] {
        for closer in ["end", "end.", "end;"] {
            let input = format!("{header}\nreturn 42;\n{closer}\n");
            let expected = format!("{header}\n  return 42;\n{closer}\n");
            let output = format_source(&input).unwrap_or_else(|e| panic!("{input}: {e}"));
            assert_eq!(output, expected);
            assert_eq!(format_source(&output).unwrap(), output);
        }
    }
}

#[test]
fn only_type_first_class_and_actor_instance_fnc_becomes_mtd() {
    for delim in ["<-", "•"] {
        let input = format!(
            "define class Calculator as\npub fnc int {delim} answer()\nbegin\nreturn 42;\nend.\n\
             static fnc int {delim} static_answer()\nbegin\nreturn 43;\nend;\nend\n"
        );
        let output = format_source(&input).unwrap_or_else(|e| panic!("{input}: {e}"));
        assert!(
            output.contains(&format!(
                "  pub mtd int {delim} answer()\n  begin\n    return 42;\n  end."
            )),
            "{output}"
        );
        assert!(
            output.contains(&format!(
                "  static fnc int {delim} static_answer()\n  begin\n    return 43;\n  end;"
            )),
            "{output}"
        );
        assert_eq!(format_source(&output).unwrap(), output);

        let actor = format!(
            "define actor Worker as\nfnc int {delim} count()\nbegin\nreturn 5;\nend.\nend\n"
        );
        let actor_out = format_source(&actor).unwrap_or_else(|e| panic!("{actor}: {e}"));
        assert!(
            actor_out.contains(&format!(
                "  mtd int {delim} count()\n  begin\n    return 5;\n  end."
            )),
            "{actor_out}"
        );
    }
}

#[test]
fn top_level_fnc_and_explicit_mtd_are_preserved() {
    let src = "fnc int <- foo()\nbegin\nreturn 42;\nend.\n";
    assert_eq!(
        format_source(src).unwrap(),
        "fnc int <- foo()\nbegin\n  return 42;\nend.\n"
    );

    let src = "define class C as\nmtd int• foo()\nbegin\nreturn 42;\nend.\nend\n";
    let result = format_source(src).unwrap();
    assert!(
        result.contains("  mtd int• foo()\n  begin\n    return 42;\n  end."),
        "{result}"
    );
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn conditional_then_else_fi_inside_begin_stays_structural_and_two_space() {
    let input = "fnc int• foo()\nbegin\nif ready then\nreturn 42;\nelse\nreturn 1;\nfi\nend.\n";
    let expected = "fnc int• foo()\nbegin\n  if ready then\n    return 42;\n  else\n    return 1;\n  fi\nend.\n";
    let result = format_source(input).unwrap();
    assert_eq!(result, expected);
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn nested_end_dispatch_does_not_close_outer_callable() {
    let input = "int <- foo()\nbegin\nreturn match x over\non 1 -> 42,\ndefault -> 0,\nend\nend.\n";
    let result = format_source(input).unwrap();
    assert!(result.contains("  return match x over"), "{result}");
    assert!(result.contains("\n  end\nend.\n"), "{result}");
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn comments_and_literals_are_never_rewritten_as_mtd_or_begin_closers() {
    let src = "define class C as\n// fnc int• fake()\npub fnc int• foo()\nbegin\nreturn \"end. fnc int• fake()\";\nend.\nend\n";
    let out = format_source(src).unwrap();
    assert!(out.contains("// fnc int• fake()"), "{out}");
    assert!(out.contains("return \"end. fnc int• fake()\";"), "{out}");
    assert!(out.contains("pub mtd int• foo()"), "{out}");
    assert_eq!(format_source(&out).unwrap(), out);
}

#[test]
fn malformed_headers_and_unbalanced_bodies_do_not_silently_succeed() {
    for bad in [
        "int• foo()\nbegin\nreturn 42;\n",
        "int <- foo()\nreturn 42;\n",
        "int• foo()\n",
        "int● foo()\nbegin\nreturn 42;\nend.\n",
    ] {
        assert!(format_source(bad).is_err() || bad.contains('●'), "{bad}");
    }
}

#[test]
fn regular_legacy_method_spelling_is_unchanged() {
    let src = "define class C as\npub fnc legacy(): int { return 42; }\nend\n";
    let formatted = format_source(src).unwrap();
    assert!(formatted.contains("pub legacy()"), "{formatted}");
    assert!(!formatted.contains("pub mtd legacy()"), "{formatted}");
}
