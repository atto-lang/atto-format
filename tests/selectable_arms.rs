use oreslang_format::{format_source, is_formatted};

#[test]
fn selectable_arms_mix_channels_observables_futures_and_custom_sources() {
    let source = r#"do select {
case readch foo: int i -> { use(i); }
when obs.take(1): int a -> { use(a); }
when await fut: const int b -> { use(b); }
when mySelectable: const int mut c -> { use(c); }
when writech out, 4: { sent(); }
}
"#;
    let formatted = format_source(source).unwrap();
    for header in [
        "when readch foo: int i -> {",
        "when obs.take(1): int a -> {",
        "when await fut: const int b -> {",
        "when mySelectable: const int mut c -> {",
        "when writech out, 4: {",
    ] {
        assert!(formatted.contains(header), "missing {header}: {formatted}");
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn selectable_binding_preserves_nested_generic_type_and_comments() {
    let source = r#"select first {
when stream.next(): Option<Tuple<int, string>> pair -> { // a comment
consume(pair);
}
when await fut: val x { consume(x); }
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(
        formatted.contains("when stream.next(): Option<Tuple<int, string>> pair -> { // a comment")
    );
    assert!(formatted.contains("when await fut: val x {"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn unknown_or_unbounded_selectable_arms_fail_closed() {
    for source in [
        "select { when unknown source: {} }",
        "select { when obs.next(): int x -> }",
        "select { when obs.next(): int x -> eat(x); }",
        "select { when obs.next(): int x -> (broken] {} }",
    ] {
        assert!(format_source(source).is_err(), "{source}");
    }
}

#[test]
fn legacy_unbraced_read_arm_stops_before_typed_custom_selectable_arm() {
    let input = r#"select {
case readch input: val x
consume(x);
when obs.take(1): int a -> { consume(a); }
}
"#;
    let formatted = format_source(input).unwrap();
    assert!(
        formatted.contains("when readch input: val x {"),
        "{formatted}"
    );
    assert!(
        formatted.contains("consume(x);\n  }\n  when obs.take(1): int a -> {"),
        "{formatted}"
    );
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn ordinary_when_identifier_inside_unbraced_body_is_not_a_new_selectable_arm() {
    let input = "select { case readch foo: val value\nwhen = value;\nconsume(when);\ndefault: }\n";
    let formatted = format_source(input).unwrap();
    assert!(formatted.contains("when = value;"), "{formatted}");
    assert!(formatted.contains("consume(when);"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn selectable_void_arm_has_explicit_arrow_but_no_value_binding() {
    let source = "select { when self.outbox.send(command): -> { sent(); } }\n";
    let formatted = format_source(source).unwrap();
    assert!(
        formatted.contains("when self.outbox.send(command): -> {"),
        "{formatted}"
    );
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
