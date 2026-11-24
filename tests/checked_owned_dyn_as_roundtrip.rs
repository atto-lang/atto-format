use oreslang_format::{format_source, is_formatted};

// A formatter does not own cast semantics. It must preserve the checked as
// spelling, the explicit take! ownership action, and the constructor/contextual
// alternatives byte-for-byte at the expression level.
#[test]
fn checked_dyn_erasure_all_forms_preserve_ownership_and_are_idempotent() {
    let source = r#"
pub fnc main(): int {
val Dyn<Printable> a = Dyn<Printable>(take! x);
val Dyn<Printable> b = (take! y) as Dyn<Printable>;
val Dyn<Printable> c = take! z;
val Dyn<Printable> d = take! w as Dyn<Printable>;
// (take! z) as Dyn<Other> is commentary, not a conversion.
val string example = "(take! quoted) as Dyn<Printable>";
return 0;
}
"#;
    let formatted = format_source(source).expect("checked owned Dyn source must format");
    for wanted in [
        "val Dyn<Printable> a = Dyn<Printable>(take! x);",
        "val Dyn<Printable> b = (take! y) as Dyn<Printable>;",
        "val Dyn<Printable> c = take! z;",
        "val Dyn<Printable> d = take! w as Dyn<Printable>;",
        "// (take! z) as Dyn<Other> is commentary, not a conversion.",
        "\"(take! quoted) as Dyn<Printable>\"",
    ] {
        assert!(formatted.contains(wanted), "lost or changed {wanted}: {formatted}");
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn formatter_does_not_forge_a_take_into_unsafe_as_dyn_source() {
    let source = r#"pub fnc main(): int {
  val Dyn<Printable> a = concrete as Dyn<Printable>;
  val Dyn<Printable> b = (borrow! concrete) as Dyn<Printable>;
  return 0;
}
"#;
    let formatted = format_source(source).expect("formatter is not the typechecker");
    assert!(formatted.contains("concrete as Dyn<Printable>"));
    assert!(formatted.contains("(borrow! concrete) as Dyn<Printable>"));
    assert!(!formatted.contains("take! concrete"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
