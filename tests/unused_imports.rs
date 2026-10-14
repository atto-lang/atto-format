use oreslang_format::{format_source, is_formatted};

fn canonical(source: &str) -> String {
    let once = format_source(source).unwrap();
    assert_eq!(format_source(&once).unwrap(), once, "formatting must be idempotent");
    assert!(is_formatted(&once).unwrap());
    once
}

#[test]
fn comments_unused_imports_but_preserves_their_source_text() {
    let src = r#"import class Box from "./box";
import fnc build from "./factory"; // the original trailing comment
pub fnc main(): void {
  build();
  return;
}
"#;
    let formatted = canonical(src);
    assert!(formatted.contains("// import class Box from \"./box\";\n"));
    assert!(formatted.contains(
        "import fnc build from \"./factory\"; // the original trailing comment"
    ));
    assert!(!formatted.contains("// import fnc build"));
}

#[test]
fn comments_aliases_by_local_name_not_original_name() {
    let src = r#"import class ArrayList as JList from "java:java.util.ArrayList";
import fnc abs as jabs from "java:java.lang.Math";
import * as utilities from "./util";
import module * as services from "./services";
pub fnc main(): int {
  return jabs(-4);
}
"#;
    let formatted = canonical(src);
    assert!(formatted.contains("// import class ArrayList as JList"));
    assert!(formatted.contains("import fnc abs as jabs from"));
    assert!(formatted.contains("// import * as utilities"));
    assert!(formatted.contains("// import module * as services"));
}

#[test]
fn grouped_imports_remain_active_when_any_member_is_referenced() {
    let src = r#"import types (One, Two, Three) from "./types";
import fnc {run, stop} from "./other";
import types A, B, C from "./more";
pub fnc main(): void {
  val One x;
  stop();
  return;
}
"#;
    let formatted = canonical(src);
    assert!(formatted.contains("import types (One, Two, Three)"));
    assert!(formatted.contains("import fnc {run, stop}"));
    assert!(formatted.contains("// import types A, B, C"));
}

#[test]
fn ignores_appearances_in_comments_strings_paths_and_symbols() {
    let src = r#"import class Phantom from "./Phantom";
import type Hidden from "./Hidden";
pub fnc main(): void {
  // Phantom and Hidden are mentioned, but not referenced.
  val name = "Phantom Hidden";
  val symbol = #Hidden;
  return;
}
"#;
    let formatted = canonical(src);
    assert!(formatted.contains("// import class Phantom from"));
    assert!(formatted.contains("// import type Hidden from"));
}

#[test]
fn intentional_side_effect_import_is_kept_with_marker() {
    let src = r#"// oresfmt: keep-import
import module Bootstrap from "./boot";
import module NotNeeded from "./extra"; // oresfmt: keep-import
pub fnc main(): void {
  return;
}
"#;
    let formatted = canonical(src);
    assert!(formatted.contains("\nimport module Bootstrap from"));
    assert!(formatted.contains("\nimport module NotNeeded from"));
    assert!(!formatted.contains("// import module"));
}

#[test]
fn namespace_header_and_multiline_imports_are_supported() {
    let src = r#"namespace app;
import types (
  One,
  Two
) from "./types";
pub fnc main(): void {
  return;
}
"#;
    let formatted = canonical(src);
    assert!(formatted.contains("// import types ("));
    assert!(formatted.contains("// One,"));
    assert!(formatted.contains("// Two"));
    assert!(formatted.contains("// ) from \"./types\";"));
    assert!(formatted.contains("namespace app;"));
}

#[test]
fn code_sharing_import_line_is_never_swallowed_by_comment() {
    let src = "namespace app; import class X from './x';\n";
    let formatted = canonical(src);
    assert_eq!(formatted, src);
}

#[test]
fn a_lexically_used_import_stays_active_in_nested_code() {
    let src = r#"import actor Worker from "./worker";
define actor Supervisor as
  pub start(): void {
    val task = spawn Worker();
    return;
  }
end
"#;
    let formatted = canonical(src);
    assert!(formatted.starts_with("import actor Worker"));
    assert!(!formatted.contains("// import actor Worker"));
}
