//! Regression for explicit out-of-line receiver methods that terminate with end.
//! Their first parenthesized group is the receiver; the second is the method
//! parameter list. Neither comments nor random expressions can open frames.

use oreslang_format::format_source;

#[test]
fn production_immutable_class_mtd_fixture_formats_and_is_idempotent() {
    let original = include_str!("fixtures/immutable-class-mtd.atto");
    let formatted = format_source(original).expect("two out-of-line mtd bodies must own their end");
    assert!(formatted.contains("pub mtd (Calculator _) four() -> int"));
    assert!(formatted.contains("pub mtd (self: Calculator) five() -> int ::"));
    assert!(formatted.contains("return calculator.one()"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn receiver_methods_use_second_parameter_group_for_end_ownership() {
    for signature in [
        "pub mtd (self: Calculator) sum(int value) -> int",
        "pub mtd (Calculator receiver) sum(int value) -> int ::",
    ] {
        let src = format!("define class Calculator as\nend\n{signature}\n  return 4;\nend\n");
        let output = format_source(&src).unwrap_or_else(|err| panic!("{signature}: {err}"));
        assert!(output.contains("return 4;"));
        assert_eq!(format_source(&output).unwrap(), output);
    }
}

#[test]
fn malformed_receiver_or_signature_cannot_open_a_spurious_end_frame() {
    for signature in [
        "pub mtd (Calculator receiver) -> int",
        "pub mtd (Calculator) sum() -> int;",
        "pub mtd (Calculator) sum() => int",
    ] {
        let source = format!("define class Calculator as\nend\n{signature}\nend\n");
        assert!(format_source(&source).is_err(), "{signature} should not own an end frame");
    }
}
