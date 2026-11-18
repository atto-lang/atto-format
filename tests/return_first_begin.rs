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
