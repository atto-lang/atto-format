use oreslang_format::{format_source, is_formatted};

#[test]
fn preserves_case_and_when_as_ordinary_identifiers() {
    let src = r#"fnc sum(int case, int when) -> int {
  const total = case + when;
  return total;
}
"#;

    assert_eq!(format_source(src).unwrap(), src);
    assert!(is_formatted(src).unwrap());
}

#[test]
fn canonicalizes_only_the_select_arm_case_keyword() {
    let src = r#"fnc receive(Channel<int> input, int when) -> void {
  do nb select {
    case readch input: val case {
      stdio.println(case, when);
    }
  }
  return;
}
"#;

    let expected = r#"fnc receive(Channel<int> input, int when) -> void {
  do nb select {
    when readch input: val case {
      stdio.println(case, when);
    }
  }
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn switch_case_keyword_and_case_subject_identifier_are_not_rewritten() {
    let src = r#"fnc classify(int case, int when) -> void {
  switch case;
    case 1 -> {
      stdio.println(when);
    }
    default -> {
    }
  end
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), src);
    assert!(is_formatted(src).unwrap());
}

#[test]
fn nested_switches_keep_their_own_end_and_are_idempotent() {
    let src =
        "define module M as\nswitch 1;\ncase 1 -> {\nswitch 2\ncase 2 -> {}\nend\n}\nend\nend\n";
    let expected = "define module M as\n  switch 1;\n    case 1 -> {\n      switch 2\n        case 2 -> {}\n      end\n    }\n  end\nend\n";
    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn switch_closers_cannot_cross_unclosed_braces() {
    assert!(format_source("switch 1;\ncase 1 -> {\nend\n}\n").is_err());
    assert!(format_source("fnc f() -> void {\nswitch 1;\n}\nend\n").is_err());
    assert!(format_source("switch 1;\ncase 1 -> {}\n").is_err());
}

#[test]
fn switch_text_in_comments_and_literals_does_not_open_a_block() {
    let src = "// switch case;\nconst message = \"switch when;\";\n/* switch 1; */\n";
    assert_eq!(format_source(src).unwrap(), src);
}

#[test]
fn contextual_words_are_valid_select_channel_and_value_names() {
    let src = "select { case readch case: val when {} case writech when, case: {} }\n";
    let formatted = format_source(src).unwrap();
    assert!(formatted.contains("when readch case: val when {"));
    assert!(formatted.contains("when writech when, case: {"));
    assert!(is_formatted(&formatted).unwrap());
}
