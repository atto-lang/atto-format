use oreslang_format::{format_source, is_formatted};

#[test]
fn validation_shape_wraps_when_a_three_parameter_name_has_two_underscores() {
    let input = r#"define module cni_validation as
  pub fnc valid_mtu(int mtu, int minimum_mtu, int maximum_mtu): bool {
    return mtu >= minimum_mtu & mtu <= maximum_mtu & minimum_mtu > 0;
  }
  pub fnc may_attach(bool authorized_namespace, bool policy_accepted,
                     bool sandbox_identity_valid): bool {
    return authorized_namespace & policy_accepted & sandbox_identity_valid;
  }
end
"#;

    let expected = r#"define module cni_validation as
  pub fnc valid_mtu(int mtu, int minimum_mtu, int maximum_mtu) -> bool {
    return mtu >= minimum_mtu & mtu <= maximum_mtu & minimum_mtu > 0;
  }
  pub fnc may_attach(
    bool authorized_namespace,
    bool policy_accepted,
    bool sandbox_identity_valid
  ) -> bool {
    return authorized_namespace & policy_accepted & sandbox_identity_valid;
  }
end
"#;

    let got = format_source(input).unwrap();
    assert_eq!(got, expected);
    assert_eq!(format_source(&got).unwrap(), got);
    assert!(is_formatted(&got).unwrap());
    assert!(!got.contains('\t'));
}

#[test]
fn four_parameters_always_wrap_one_per_line() {
    let input = "fnc add(int a, int b, int c, int d): int {\nreturn a + b + c + d;\n}\n";
    let expected = r#"fnc add(
  int a,
  int b,
  int c,
  int d
) -> int {
  return a + b + c + d;
}
"#;

    let got = format_source(input).unwrap();
    assert_eq!(got, expected);
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn three_parameters_wrap_only_above_sixty_total_name_columns() {
    let at_sixty = "fnc sixty(int aaaaaaaaaaaaaaaaaaaa, int bbbbbbbbbbbbbbbbbbbb, int cccccccccccccccccccc): void {\nreturn;\n}\n";
    let expected_sixty = "fnc sixty(int aaaaaaaaaaaaaaaaaaaa, int bbbbbbbbbbbbbbbbbbbb, int cccccccccccccccccccc) -> void {\n  return;\n}\n";
    assert_eq!(format_source(at_sixty).unwrap(), expected_sixty);

    let above_sixty = "fnc sixty_one(int aaaaaaaaaaaaaaaaaaaa, int bbbbbbbbbbbbbbbbbbbb, int ccccccccccccccccccccc): void {\nreturn;\n}\n";
    let expected_above_sixty = r#"fnc sixty_one(
  int aaaaaaaaaaaaaaaaaaaa,
  int bbbbbbbbbbbbbbbbbbbb,
  int ccccccccccccccccccccc
) -> void {
  return;
}
"#;
    let got = format_source(above_sixty).unwrap();
    assert_eq!(got, expected_above_sixty);
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn fewer_than_three_parameters_do_not_wrap_from_underscore_names() {
    let input =
        "fnc pair(bool namespace_policy_accepted, bool sandbox_identity_valid): bool {\nreturn true;\n}\n";
    let expected =
        "fnc pair(bool namespace_policy_accepted, bool sandbox_identity_valid) -> bool {\n  return true;\n}\n";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn instance_methods_use_the_same_two_space_continuation_rule() {
    let input = r#"define class Driver as
  attach(bool a, bool b, bool c, bool d): void {
    return;
  }
end
"#;
    let expected = r#"define class Driver as
  attach(
    bool a,
    bool b,
    bool c,
    bool d
  ) -> void {
    return;
  }
end
"#;

    let got = format_source(input).unwrap();
    assert_eq!(got, expected);
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn end_delimited_functions_remain_idempotent_after_wrapping() {
    let input = r#"fnc add(int a, int b, int c, int d) -> int
return a + b + c + d;
end
"#;
    let expected = r#"fnc add(
  int a,
  int b,
  int c,
  int d
) -> int
  return a + b + c + d;
end
"#;

    let got = format_source(input).unwrap();
    assert_eq!(got, expected);
    assert_eq!(format_source(&got).unwrap(), got);
}
