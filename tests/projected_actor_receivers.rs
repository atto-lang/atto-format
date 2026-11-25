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

#[test]
fn named_receiver_and_body_binding_survive_production_formatter() {
    for binding in ["this", "node"] {
        let src = format!(
            "declare actor Foo;\nmtd (Foo mut {binding}.state.{{counter, xyz}}) update(): void {{\n  {binding}.handle_state(critical-write |s| -> {{ s.counter += 1; }});\n}}\n"
        );
        let output = format_source(&src).expect("named projected receiver must format");
        assert!(output.contains(&format!("{binding}.state.")), "{output}");
        assert!(output.contains(&format!("{binding}.handle_state")), "{output}");
        assert_eq!(format_source(&output).unwrap(), output);
    }
}

#[test]
fn named_plain_receiver_is_not_rewritten_as_self() {
    let src = "declare actor Foo;\nmtd (Foo this) get_id(): int { return 1; }\n";
    let output = format_source(src).expect("named receiver must format");
    assert!(output.contains("this"), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
}
