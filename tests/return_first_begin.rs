use oreslang_format::{format_source, is_formatted};

#[test]
fn formats_all_four_return_first_function_spellings_with_two_spaces() {
    let input = "int <- foo()\nbegin\n    return 42;\nend.\nint• bar()\nbegin\nreturn 42;\nend.\nfnc int <- baz()\nbegin\n  return 42;\nend;\nfnc int• qux()\nbegin\nreturn 42;\nend\n";
    let expected = "int <- foo()\nbegin\n  return 42;\nend.\n\n\nint• bar()\nbegin\n  return 42;\nend.\n\n\nfnc int <- baz()\nbegin\n  return 42;\nend;\n\n\nfnc int• qux()\nbegin\n  return 42;\nend\n";
    let got = format_source(input).expect("format return-first functions");
    assert_eq!(got, expected);
    assert!(is_formatted(&got).unwrap());
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn canonicalizes_instance_fnc_to_mtd_without_changing_free_or_static_fnc() {
    let input = r#"fnc int <- top()
begin
return 42;
end.

define class Counter as
fnc int <- legacy()
begin
return 42;
end.
mtd int• canonical()
begin
return 42;
end;
int <- bare()
begin
return 42;
end
static fnc helper(): int {
return 42;
}
end
"#;
    let got = format_source(input).expect("format nested begin/end methods");
    assert!(got.contains("fnc int <- top()\nbegin\n  return 42;\nend."));
    assert!(got.contains("  mtd int <- legacy()\n  begin\n    return 42;\n  end."));
    assert!(got.contains("  mtd int• canonical()\n  begin\n    return 42;\n  end;"));
    assert!(got.contains("  int <- bare()\n  begin\n    return 42;\n  end"));
    assert!(got.contains("  static fnc helper()"), "{got}");
    assert!(!got.contains("  fnc int <- legacy()"));
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn nested_keyword_blocks_indentation_and_quoted_markers_are_preserved() {
    let input = r#"string <- classify()
begin
if true then
return "begin end. <- •";
else
return "no";
fi
end.
"#;
    let got = format_source(input).unwrap();
    assert!(got.contains("begin\n  if true then\n    return \"begin end. <- •\";\n  else\n    return \"no\";\n  fi\nend."), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn rejects_missing_begin_and_unmatched_ends() {
    for input in [
        "int <- f()\nreturn 1;\nend",
        "int• f()",
        "int <- f()\nbegin\nreturn 1;",
        "int <- f()\nbegin\nreturn 1;\nend.\nend",
    ] {
        assert!(format_source(input).is_err(), "{input}");
    }
}

#[test]
fn return_first_generic_method_and_default_argument_are_recognized() {
    let input = r#"fnc Result<int, string> <- wrap<T>(T value)
begin
  return Ok(42);
end

define class Generic as
fnc int <- choose<T>(T value, int fallback = 2)
begin
return fallback;
end.
end
"#;
    let got = format_source(input).unwrap();
    assert!(got.contains("fnc Result<int, string> <- wrap<T>(T value)\nbegin\n  return Ok(42);\nend"), "{got}");
    assert!(got.contains("  mtd int <- choose<T>(T value, int fallback = 2)\n  begin\n    return fallback;\n  end."), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn static_return_first_remains_fnc_not_mtd() {
    let input = "define class Helpers as\nstatic fnc int <- utility()\nbegin\nreturn 42;\nend;\nend\n";
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("  static fnc int <- utility()\n  begin\n    return 42;\n  end;"), "{formatted}");
    assert!(!formatted.contains("static mtd"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn structural_return_type_braces_do_not_open_a_fake_function_body() {
    let input = r#"fnc struct {value: int} <- shape()
begin // keep opener
return value;
end. // keep closer

define class Shapes as
fnc record {value: int}• current()
begin
return value;
end;
end
"#;
    let got = format_source(input).unwrap();
    assert!(got.contains("fnc struct {value: int} <- shape()\nbegin // keep opener\n  return value;\nend. // keep closer"), "{got}");
    assert!(got.contains("  mtd record {value: int}• current()\n  begin\n    return value;\n  end;"), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn begin_must_stay_a_distinct_line_and_existing_arrows_keep_their_meaning() {
    for malformed in [
        "int <- foo()\nbegin return 42;\nend",
        "int <- foo()\nbegin;\n  return 42;\nend",
        "int <- foo()\nbegin\n  return 42;",
    ] {
        assert!(format_source(malformed).is_err(), "{malformed}");
    }
    let comparison = "fnc compare() -> bool {\nreturn 1<-2;\n}\n";
    let got = format_source(comparison).unwrap();
    assert!(got.contains("return 1<-2;"), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn checked_effects_and_nested_parenthesized_arguments_do_not_confuse_header() {
    let src = r#"fnc int <- run(int n) throws Error
begin
  return n;
end

define class C as
fnc int <- use(int n = compute(1, 2)) raises(Error)
begin
  return n;
end;
end
"#;
    let formatted = format_source(src).unwrap();
    assert!(formatted.contains("fnc int <- run(int n) throws Error\nbegin\n  return n;\nend"), "{formatted}");
    assert!(formatted.contains("  mtd int <- use(int n = compute(1, 2)) raises(Error)\n  begin\n    return n;\n  end;"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn optional_mtd_migration_handles_bare_and_existing_return_first_methods() {
    let source = r#"define class C as
int <- bare()
begin
  return 42;
end
mtd int• explicit()
begin
  return 42;
end;
end
"#;
    let default = format_source(source).unwrap();
    assert!(default.contains("  int <- bare()\n  begin"), "{default}");
    assert!(default.contains("  mtd int• explicit()\n  begin"), "{default}");
    let migrated = oreslang_format::format_source_with_mtd(source).unwrap();
    assert!(migrated.contains("  mtd int <- bare()\n  begin"), "{migrated}");
    assert!(migrated.contains("  mtd int• explicit()\n  begin"), "{migrated}");
    assert!(!migrated.contains("mtd mtd"), "{migrated}");
    assert_eq!(
        oreslang_format::format_source_with_mtd(&migrated).unwrap(),
        migrated
    );
}
