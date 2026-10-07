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
