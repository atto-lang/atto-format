use oreslang_format::{format_source, is_formatted};

#[test]
fn formats_end_delimited_select_wait_modes_and_is_idempotent() {
    let source = r#"fnc forms() -> void {
select first over
when readch input: val value -> {
consume(value);
}
default: {
fallback();
}
end

do select fair over
default: {
side_effect();
}
end

nb select random over
case await future: int value -> {
use(value);
}
default: {
later();
}
end

do nb select over
case readch inbox: int value -> {
use(value);
}
default: {
later_side_effect();
}
end
return;
}
"#;

    let expected = r#"fnc forms() -> void {
  select first over
    when readch input: val value -> {
      consume(value);
    }
    default: {
      fallback();
    }
  end

  do select fair over
    default: {
      side_effect();
    }
  end

  nb select random over
    when await future: int value -> {
      use(value);
    }
    default: {
      later();
    }
  end

  do nb select over
    when readch inbox: int value -> {
      use(value);
    }
    default: {
      later_side_effect();
    }
  end
  return;
}
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn nested_over_end_selects_keep_end_ownership() {
    let source = r#"fnc nested() -> void {
select over
default: {
do nb select over
default: {
work();
}
end
}
end
return;
}
"#;

    let expected = r#"fnc nested() -> void {
  select over
    default: {
      do nb select over
        default: {
          work();
        }
      end
    }
  end
  return;
}
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn try_select_over_uses_the_same_end_scope_when_present() {
    let source = r#"fnc probe() -> void {
try select over
default: {
return;
}
end
}
"#;
    let expected = r#"fnc probe() -> void {
  try select over
    default: {
      return;
    }
  end
}
"#;
    assert_eq!(format_source(source).unwrap(), expected);
}
