//! Value-based maybe uses its own end and else scope, independent of try/if.
//! Parser/runtime admission belongs to the individual compilers; these are
//! pure formatter structural and idempotence regressions.
use oreslang_format::{format_source, is_formatted};

#[test]
fn return_maybe_inside_keyword_try_preserves_nested_end_and_finally() {
    let source = r#"fnc main(): int {
try
return maybe
Ok(value) ?= fetch();
Ok(value)
else
Err(reason) -> { return 0; };
end;
catch()
return 1;
finally
log();
end
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("\n  try\n    return maybe\n      Ok(value) ?= fetch();"));
    assert!(formatted.contains("\n    else\n      Err(reason) -> {"));
    assert!(formatted.contains("\n    end;\n  catch()\n    return 1;"));
    assert!(formatted.contains("\n  finally\n    log();\n  end\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn do_maybe_scopes_else_separately_from_nested_if() {
    let source = r#"fnc main(): void {
do maybe
Ok(value) ?= fetch();
if ready then
observe(value);
else
warn();
fi
else
Err(reason) -> { log(reason); };
end
return;
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("\n  do maybe\n    Ok(value) ?= fetch();"));
    assert!(formatted.contains("\n    if ready then\n      observe(value);\n    else\n      warn();\n    fi\n"));
    assert!(formatted.contains("\n  else\n    Err(reason) -> {"));
    assert!(formatted.contains("\n  end\n  return;"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn contextual_identifier_named_maybe_is_not_a_keyword_block() {
    let input = "fnc main(): int {\nlet maybe = 42;\nreturn maybe;\n}\n";
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("return maybe;"));
    assert!(!formatted.contains("\n  end\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn missing_or_duplicate_maybe_else_fails_closed() {
    for source in [
        "fnc main(): void {\ndo maybe\nOk(v) ?= fetch();\nend\n}\n",
        "fnc main(): int {\nreturn maybe\nOk(v) ?= fetch();\nend;\n}\n",
        "fnc main(): void {\ndo maybe\nOk(v) ?= fetch();\nelse\nErr(e) -> 0;\nelse\nErr(e) -> 1;\nend\n}\n",
        "fnc main(): void {\ndo maybe\nOk(v) ?= fetch();\nelif wrong then\nend\n}\n",
    ] {
        assert!(format_source(source).is_err(), "must reject: {source}");
    }
}

#[test]
fn comment_and_literal_else_end_do_not_close_maybe() {
    let source = r#"fnc main(): void {
do maybe // open scope
Ok(v) ?= fetch();
log("else end");
else // value handler
Err(e) -> { log("try end"); };
end
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("log(\"else end\");"));
    assert!(formatted.contains("\n  else // value handler\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
