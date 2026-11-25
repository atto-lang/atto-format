use oreslang_format::{format_source, is_formatted};

#[test]
fn keyword_try_nested_arms_indent_and_round_trip() {
    let source = r#"fnc main(): void {
try
try
stdio.println("body");
finally
stdio.println("inner");
end
catch()
stdio.println("caught");
finally
stdio.println("outer");
end
return;
}
"#;
    let expected = r#"fnc main() -> void {
  try
    try
      stdio.println("body");
    finally
      stdio.println("inner");
    end
  catch()
    stdio.println("caught");
  finally
    stdio.println("outer");
  end
  return;
}
"#;
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn braced_try_is_not_rewritten_to_keyword_style() {
    let source = r#"fnc main(): void {
try {
stdio.println("body");
} catch() {
stdio.println("caught");
} finally {
stdio.println("cleanup");
}
return;
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("try {"));
    assert!(formatted.contains("} catch() {"));
    assert!(formatted.contains("} finally {"));
    assert!(!formatted.contains("\n  end\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn keyword_end_in_comments_or_literals_is_not_a_terminator() {
    let source = r#"fnc main(): void {
try
stdio.println("end catch finally"); // end catch finally
finally
stdio.println("done");
end
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("    stdio.println(\"end catch finally\"); // end catch finally"));
    assert!(formatted.contains("\n  finally\n"));
    assert!(formatted.contains("\n  end\n"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn missing_keyword_end_is_rejected() {
    let source = "fnc main(): void {\ntry\n stdio.println(1);\nfinally\n stdio.println(2);\n}\n";
    assert!(format_source(source).is_err());
}
