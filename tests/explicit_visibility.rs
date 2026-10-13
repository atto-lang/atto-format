use oreslang_format::format_source;

#[test]
fn public_stays_explicit_and_define_precedes_pub() {
    let source = r#"
pub define class PublicBox as
  val int hidden: 1
  pub fnc visible(): int { return 2; }
end

pub define trait API as
end

pub define struct Point as
end

pub define type UserId = int;
pub type LocalId = int;
type HiddenId = int;
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("define pub class PublicBox as"));
    assert!(formatted.contains("define pub trait API as"));
    assert!(formatted.contains("define pub struct Point as"));
    assert!(formatted.contains("define pub type UserId = int;"));
    assert!(formatted.contains("pub type LocalId = int;"));
    assert!(formatted.contains("type HiddenId = int;"));
    assert!(formatted.contains("val int hidden: 1"));
    assert!(!formatted.contains("pub val int hidden"));
    assert!(!formatted.contains("pub define"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn bare_named_declarations_keep_define_optional_and_indent_correctly() {
    let source = r#"pub class Box as
val int hidden: 1
end

trait Descriptor as
end

struct Position as
end

define class Vault as
val int secret: 3
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("pub class Box as\n  val int hidden: 1\nend"));
    assert!(formatted.contains("trait Descriptor as\nend"));
    assert!(formatted.contains("struct Position as\nend"));
    assert!(formatted.contains("define class Vault as\n  val int secret: 3\nend"));
    assert!(!formatted.contains("define pub class Vault"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn comments_and_literals_cannot_trigger_visibility_rewrite() {
    let source = r#"pub /* keep position */ define class Example as
  val String text = "pub define type Password = String";
end
// pub define type Comment = String;
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("pub /* keep position */ define class Example as"));
    assert!(formatted.contains("\"pub define type Password = String\""));
    assert!(formatted.contains("// pub define type Comment = String;"));
}
