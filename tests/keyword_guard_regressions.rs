use oreslang_format::format_source;

#[test]
fn compact_guards_keep_their_delimiters_and_statement_boundaries() {
    for (guard, expected_guard) in [
        (
            "guard ready { record_failure(); return false; }",
            "  guard ready {\n    record_failure();\n    return false;\n  }",
        ),
        (
            "guard ready else record_failure(); return false end",
            "  guard ready else\n    record_failure();\n    return false;\n  end",
        ),
        (
            "guard ready; else record_failure(); return false; end;",
            "  guard ready; else\n    record_failure();\n    return false;\n  end;",
        ),
    ] {
        let source = format!("fnc run() -> bool {{ {guard} return true; }}\n");
        let expected = format!("fnc run() -> bool {{\n{expected_guard}\n  return true;\n}}\n");
        let output = format_source(&source).unwrap();
        assert_eq!(output, expected);
        assert_eq!(format_source(&output).unwrap(), output);
    }
}

#[test]
fn guards_inside_keyword_if_leave_the_outer_else_in_place() {
    for (guard, expected_guard) in [
        (
            "guard ready { log(\"else; end }\"); return; }",
            "    guard ready {\n      log(\"else; end }\");\n      return;\n    }",
        ),
        (
            "guard ready else log(\"else; end }\"); return; end",
            "    guard ready else\n      log(\"else; end }\");\n      return;\n    end",
        ),
    ] {
        let source =
            format!("fnc run() {{\nif enabled then {guard} work(); else fallback(); fi\n}}\n");
        let expected = format!(
            "fnc run() {{\n  if enabled then\n{expected_guard}\n    work();\n  else\n    fallback();\n  fi\n}}\n"
        );
        let output = format_source(&source).unwrap();
        assert_eq!(output, expected);
        assert_eq!(format_source(&output).unwrap(), output);
    }
}

#[test]
fn braced_if_keeps_its_else_after_a_guard() {
    let source = "fnc run() {\nif enabled {guard ready else { return; } work();} else {\nfallback();\n} fi\n}\n";
    let expected = "fnc run() {\n  if enabled {\n    guard ready else {\n      return;\n    }\n    work();\n  } else {\n    fallback();\n  } fi\n}\n";
    let output = format_source(source).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn braced_if_inside_keyword_guard_keeps_both_if_closers() {
    let source = "fnc run() {\nif enabled then\nguard ready else\nif retry {\nretry_work();\n} fi\nreturn;\nend\nelse fallback(); fi\n}\n";
    let expected = "fnc run() {\n  if enabled then\n    guard ready else\n      if retry {\n        retry_work();\n      } fi\n      return;\n    end\n  else\n    fallback();\n  fi\n}\n";
    let output = format_source(source).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn nested_guards_keep_their_own_return_and_closer() {
    let source = "fnc run() -> bool {\nguard ready else // unready\nguard retry {return false;}\nreturn false;\nend\nreturn true;\n}\n";
    let expected = "fnc run() -> bool {\n  guard ready else // unready\n    guard retry {\n      return false;\n    }\n    return false;\n  end\n  return true;\n}\n";
    let output = format_source(source).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn a_contextual_guard_call_leaves_the_if_else_in_place() {
    let source = "fnc run() {\nif enabled then guard(); else fallback(); fi\n}\n";
    let expected =
        "fnc run() {\n  if enabled then\n    guard();\n  else\n    fallback();\n  fi\n}\n";
    let output = format_source(source).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn each_guard_requires_its_own_outermost_return() {
    for body in [
        "guard ready { if failed then return; fi }",
        "guard ready else guard retry { return; } end",
        "guard ready else report_failure(); end",
    ] {
        let source = format!("fnc run() {{\n{body}\nreturn;\n}}\n");
        let error = format_source(&source).unwrap_err();
        assert!(error.message().contains("outermost scope"), "{error}");
    }
}

#[test]
fn identifier_functions_named_fi_and_else_remain_calls() {
    let source = "fnc f() {\nif ready then fi(); else(); fi\n}\n";
    let output = format_source(source).unwrap();
    assert!(output.contains("fi();"));
    assert!(output.contains("else();"));
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn return_continuations_after_blank_lines_are_not_separated() {
    let source = "fnc f() {\nreturn obj\n\n  .method()\n}\n";
    let output = format_source(source).unwrap();
    assert!(!output.contains("return obj;"));
}

#[test]
fn preserves_explicit_conditional_separator() {
    let input = "fnc check() {\nif self.built; then return Err(\"consumed\"); fi\n}\n";
    let output = format_source(input).unwrap();
    assert!(output.contains("if self.built; then"), "{output}");
    assert!(output.contains("return Err(\"consumed\");"), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn comments_and_blank_lines_do_not_hide_return_continuations() {
    let input = "fnc read() {\nreturn value\n\n// explanation\n  .decode()\n}\n";
    let output = format_source(input).unwrap();
    assert!(!output.contains("return value;"), "{output}");
}

#[test]
fn single_line_return_without_continuation_gets_terminator() {
    let input = "fnc value() {\nreturn compute(1)\n}\n";
    let output = format_source(input).unwrap();
    assert!(output.contains("return compute(1);"), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn statements_on_later_if_body_lines_are_always_separated() {
    let input = "fnc run() {\nif ready; then\nfirst(); second();\nfi\n}\n";
    let expected = "fnc run() {\n  if ready; then\n    first();\n    second();\n  fi\n}\n";
    let output = format_source(input).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn nested_if_later_line_statements_are_separated() {
    let input = "fnc run() {\nif outer; then\nif inner; then\none(); two();\nfi\nfi\n}\n";
    let expected = "fnc run() {\n  if outer; then\n    if inner; then\n      one();\n      two();\n    fi\n  fi\n}\n";
    let output = format_source(input).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn logical_word_operators_are_preserved_as_expression_tokens() {
    let input = "fnc choose(bool a, bool b, bool c): bool {\nreturn a and b or c;\n}\n";
    let expected = "fnc choose(bool a, bool b, bool c) -> bool {\n  return a and b or c;\n}\n";
    let output = format_source(input).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn capitalized_boolean_builtins_are_not_confused_with_word_operators() {
    let input =
        "fnc choose(bool a, bool b, bool c): bool {\nreturn Or(Xor(a, b), And(a, b, c));\n}\n";
    let expected =
        "fnc choose(bool a, bool b, bool c) -> bool {\n  return Or(Xor(a, b), And(a, b, c));\n}\n";
    let output = format_source(input).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}
#[test]
fn xor_keyword_and_mixed_logical_spelling_round_trip_unchanged() {
    let input = "fnc combine(bool a, bool b, bool c): bool {\nreturn a xor b ^^ c and a or b;\n}\n";
    let expected =
        "fnc combine(bool a, bool b, bool c) -> bool {\n  return a xor b ^^ c and a or b;\n}\n";
    let output = format_source(input).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn terminal_rest_bool_parameter_syntax_round_trips() {
    let input = "pub fnc Or(bool a, bool b, rest ...bool): bool {\nreturn a or b;\n}\n";
    let expected = "pub fnc Or(bool a, bool b, rest ...bool) -> bool {\n  return a or b;\n}\n";
    let output = format_source(input).unwrap();
    assert_eq!(output, expected);
    assert_eq!(format_source(&output).unwrap(), output);
}
