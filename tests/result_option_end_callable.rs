//! Regression coverage for AttoLang's alternate end-delimited free function style.
//! Formatting is syntactic only; compiler admission still needs backend gates.
use oreslang_format::format_source;

#[test]
fn result_option_function_styles_round_trip_with_and_without_double_colon() {
    let source = r#"fnc validate_port(int port) -> Result<int, string>
if port <= 0 then
return Err("port must be positive");
fi
return Ok(port);
end

fnc optional_port(bool present) -> Option<int> ::
if !present then
return None;
fi
return Some(8080);
end
"#;
    let formatted = format_source(source).expect("format end-delimited functions");
    assert!(formatted.contains("fnc validate_port(int port) -> Result<int, string>\n  if"));
    assert!(formatted.contains("fnc optional_port(bool present) -> Option<int> ::\n  if"));
    assert!(formatted.contains("    return Err(\"port must be positive\");"));
    assert!(formatted.contains("  return Ok(port);"));
    assert!(formatted.contains("    return None;"));
    assert!(formatted.contains("  return Some(8080);"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn compact_end_delimited_result_type_requires_body_separator() {
    let compact = "fnc checked(int port) -> Result<int, string> :: return Ok(port); end\n";
    let formatted = format_source(compact).expect("explicit delimiter");
    assert!(formatted.contains("-> Result<int, string> ::\n  return Ok(port);\nend"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);

    let ambiguous = "fnc checked(int port) -> Result<int, string> return Ok(port); end\n";
    assert!(format_source(ambiguous).is_err(), "missing :: cannot be guessed safely");
}
