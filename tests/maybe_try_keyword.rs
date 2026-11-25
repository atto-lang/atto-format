use oreslang_format::{format_source, is_formatted};

#[test]
fn nested_return_maybe_owns_its_end_before_outer_keyword_try() {
    let source = r#"fnc main(): void {
try
return maybe
Ok(value) ?= fetch();
Ok(value)
else
Err(reason) -> { Err(reason) };
end;
catch()
return;
finally
stdio.println("F");
end
}
"#;
    let formatted = format_source(source).unwrap();
    let expected = r#"fnc main() -> void {
  try
    return maybe
      Ok(value) ?= fetch();
      Ok(value)
    else
      Err(reason) -> { Err(reason) };
    end;
  catch()
    return;
  finally
    stdio.println("F");
  end
}
"#;
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn nested_do_maybe_owns_its_end_and_value_else_is_not_if_else() {
    let source = r#"fnc main(): void {
try
do maybe
Ok(value) ?= fetch();
stdio.println(value);
else
Err(reason) -> { stdio.println(reason); };
end
catch()
return;
finally
stdio.println("F");
end
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("    do maybe\n      Ok(value) ?= fetch();"));
    assert!(formatted.contains("\n    else\n      Err(reason)"));
    assert!(formatted.contains("\n    end\n  catch()\n"));
    assert!(formatted.contains("\n  finally\n    stdio.println(\"F\");\n  end\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn identifier_named_maybe_is_not_a_block_and_lone_else_fails() {
    let source = "fnc main(): int {\nconst int maybe = 7;\nreturn maybe;\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("return maybe;"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(format_source("fnc main(): void {\nelse\nreturn;\n}\n").is_err());
}

#[test]
fn missing_maybe_end_is_rejected_instead_of_stealing_outer_try_end() {
    let source = "fnc main(): void {\ntry\ndo maybe\nOk(value) ?= fetch();\nelse\nErr(e) -> { return; };\ncatch()\nreturn;\nfinally\nreturn;\nend\n}\n";
    assert!(format_source(source).is_err());
}
