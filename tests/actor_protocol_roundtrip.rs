//! Protocol formatters are source-only; success is not a compiler/runtime proof.
use oreslang_format::{format_source, is_formatted};

const SOURCE: &str = r#"define protocol ConnectionProtocol as
initial Disconnected
state Disconnected as
on Connect(Config) -> Connected replies ConnectedAck
on Close -> Closed replies ClosedAck
end
state Connected as
on Query(Sql) -> Connected replies QueryReply
on Subscribe(Topic) -> Connected streams DataEvent
on Close -> Closed replies ClosedAck
end
terminal state Closed
end
"#;

#[test]
fn nested_protocol_states_are_indented_but_transition_contracts_are_unchanged() {
    let formatted = format_source(SOURCE).expect("format");
    assert!(formatted.contains("\n  initial Disconnected\n"), "{formatted}");
    assert!(formatted.contains("\n  state Disconnected as\n"), "{formatted}");
    assert!(formatted.contains("\n    on Connect(Config) -> Connected replies ConnectedAck\n"), "{formatted}");
    assert!(formatted.contains("\n  state Connected as\n"), "{formatted}");
    assert!(formatted.contains("\n    on Subscribe(Topic) -> Connected streams DataEvent\n"), "{formatted}");
    assert!(formatted.contains("\n  terminal state Closed\n"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted, "idempotence");
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn protocol_messages_preserve_nominal_response_and_successor_spelling() {
    let src = r#"// define protocol Fake as
define protocol Real as
  initial Open
  state Open as
    // on Forged -> Closed replies Phantom
    on Close -> Closed replies ClosedAck
  end
  terminal state Closed
end
pub fnc marker(): String {
  return "on Query -> Unknown replies No";
}
"#;
    let fmt = format_source(src).unwrap();
    assert!(fmt.contains("// define protocol Fake as"));
    assert!(fmt.contains("// on Forged -> Closed replies Phantom"));
    assert!(fmt.contains("on Close -> Closed replies ClosedAck"));
    assert!(fmt.contains("\"on Query -> Unknown replies No\""));
    assert_eq!(format_source(&fmt).unwrap(), fmt);
}

#[test]
fn protocol_state_is_not_an_actor_mutable_state_frame() {
    let src = r#"define actor Counter as
  state {
    let int count: 0
  }
end
"#;
    let formatted = format_source(src).expect("existing state syntax");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert!(formatted.contains("state {"));
    assert!(!formatted.contains("state { as"));
}

#[test]
fn inline_actor_implementer_does_not_create_a_phantom_protocol_frame() {
    let src = format!(
        "{}\ndefine actor Connection implements ConnectionProtocol as end\n",
        SOURCE,
    );
    let formatted = format_source(&src).expect("protocol + inline actor");
    assert!(formatted.contains("define actor Connection implements ConnectionProtocol as end"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn plain_actor_with_inline_end_is_not_a_protocol_container() {
    let src = r#"define actor Connection as end
define protocol StateProtocol as
  initial Ready
  state Ready as
    on Stop -> Closed replies StopAck
  end
  terminal state Closed
end
"#;
    let formatted = format_source(src).unwrap();
    assert!(formatted.contains("define actor Connection as end"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
