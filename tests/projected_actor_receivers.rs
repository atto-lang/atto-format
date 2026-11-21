//! Projected actor state receiver formatting is lossless; a formatter does
//! not grant borrow or state-guard authority.
use oreslang_format::format_source;

#[test]
fn grouped_state_receiver_survives_production_formatter() {
    let src = r#"
define actor Foo as
end
mtd (Foo mut self.state.{counter, xyz}) update(): void {
    return;
}
"#;
    let output = format_source(src).expect("projected receiver must format");
    assert!(output.contains("self.state.{counter, xyz}"), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn single_field_receiver_remains_a_field_projection() {
    let src = "declare actor Foo;\nmtd (Foo mut self.state.counter) increment(): void { return; }\n";
    let output = format_source(src).expect("single-field receiver must format");
    assert!(output.contains("self.state.counter"), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
}
