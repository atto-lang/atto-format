use oreslang_format::{format_source, is_formatted};

#[test]
fn uses_two_blank_lines_between_sibling_functions() {
    let src = r#"fnc one() => int {
return 1;
}
fnc two(): int {
return 2;
}



pub routine main() => void {
return;
}
"#;
    let expected = r#"fnc one() -> int {
  return 1;
}


fnc two() -> int {
  return 2;
}


pub routine main() -> void {
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn expression_body_lambdas_keep_executable_slim_arrow() {
    let src = r#"fnc demo() => void {
val Fnc<int, int> one = |value| -> value * 2;
val Fnc<int, int> two = |value| ->
value * 2;
val Fnc<int, int> typed = (int value) -> value * 3;
val Fnc<int, {value: int}> wrapped = |value| -> obj{value: value};
val Fnc<int, int> block = |value| -> {
return value * 2;
};
return;
}
"#;

    let expected = r#"fnc demo() -> void {
  val Fnc<int, int> one = |value| -> value * 2;
  val Fnc<int, int> two = |value| ->
    value * 2;
  val Fnc<int, int> typed = (int value) -> value * 3;
  val Fnc<int, {value: int}> wrapped = |value| -> obj{value: value};
  val Fnc<int, int> block = |value| -> {
    return value * 2;
  };
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn expression_body_lambda_inside_call_is_not_rewritten_as_type_arrow() {
    let src = r#"fnc apply(Fnc<int, int> callback, int value) => int {
return callback(value);
}

fnc demo() => int {
return apply(|value| -> value * 2, 5);
}
"#;

    let expected = r#"fnc apply(Fnc<int, int> callback, int value) -> int {
  return callback(value);
}


fnc demo() -> int {
  return apply(|value| -> value * 2, 5);
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn multiline_expression_lambda_indents_until_its_required_semicolon() {
    let src = r#"fnc demo() -> void {
val Fnc<int, int> choose = |value| ->
value > 0
? value
: -value;
val Fnc<int, int> nested = |value| ->
apply(|| -> {
return value;
});
return;
}
"#;
    let expected = r#"fnc demo() -> void {
  val Fnc<int, int> choose = |value| ->
    value > 0
    ? value
    : -value;
  val Fnc<int, int> nested = |value| ->
    apply(|| -> {
      return value;
    });
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());

    let err =
        format_source("fnc bad() -> void {\nval Fnc<int, int> f = |value| ->\nvalue * 2\n}\n")
            .unwrap_err();
    assert!(err.message().contains("explicit ';'"));
}

#[test]
fn structured_expression_lambda_after_assignment_gets_continuation_indent() {
    let src = r#"fnc demo() -> void {
val Fnc<int, {value: int}> wrap =
|value| -> obj{value: value * 2};
val wrapped = wrap(4);
return;
}
"#;
    let expected = r#"fnc demo() -> void {
  val Fnc<int, {value: int}> wrap =
    |value| -> obj{value: value * 2};
  val wrapped = wrap(4);
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn preserves_generator_headers_and_yield_delegation() {
    let src = r#"fnc gen* sync_values(): Iterator<int> {
rt yield;
yield* inner();
return;
}

async fnc generator* async_values(): AsyncIterator<int> {
rt yield();
yield* sync_values();
return;
}

generator routine legacy_values(): int {
yield 7;
return;
}

pub gen* canonical(): Iterator<int> {
yield 8;
return;
}

// gen* fnc fake() should stay a comment
fnc generator(int value) -> int {
return value;
}
fnc math(int gen, int value) -> int {
return gen * value;
}
"#;
    // Generator headers are preserved verbatim: Java rejects a top-level
    // bare `gen*` while LLVM rejects `fnc gen*`/`generator fnc`, so no
    // rewrite between spellings is accepted by both production compilers.
    let expected = r#"fnc gen* sync_values(): Iterator<int> {
  rt cooperate;
  yield* inner();
  return;
}


async fnc generator* async_values(): AsyncIterator<int> {
  rt cooperate();
  yield* sync_values();
  return;
}


generator routine legacy_values(): int {
  yield 7;
  return;
}


pub gen* canonical(): Iterator<int> {
  yield 8;
  return;
}

// gen* fnc fake() should stay a comment
fnc generator(int value) -> int {
  return value;
}


fnc math(int gen, int value) -> int {
  return gen * value;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn else_followed_by_nested_if_is_not_collapsed_to_elif() {
    let src = r#"fnc classify(int value) -> int {
if value < 0 then
return -1;
else
if value == 0 then
return 0;
else
return 1;
fi
fi
}
"#;
    let expected = r#"fnc classify(int value) -> int {
  if value < 0 then
    return -1;
  else
    if value == 0 then
      return 0;
    else
      return 1;
    fi
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn preserves_explicit_receivers_and_native_collection_members() {
    let src = r#"define class Cursor as
pub next(mut self)() -> int {
self.index = self.index + 1;
return self.items.size;
}
end
"#;
    let expected = r#"define class Cursor as
  pub next(mut self)() -> int {
    self.index = self.index + 1;
    return self.items.size;
  }
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn preserves_canonical_pointerless_borrow_syntax() {
    let src = r#"fnc update(borrow mut Thing thing) -> void {
val view = rt mut borrow thing;
return;
}
"#;
    let formatted = format_source(src).unwrap();
    assert!(formatted.contains("borrow mut Thing thing"));
    assert!(formatted.contains("rt mut borrow thing"));
}

#[test]
fn rejects_pointer_style_ownership_syntax() {
    for source in [
        "fnc read(&Thing thing) -> int { return thing.value; }\n",
        "fnc read(borrow Thing thing) -> int { return &thing.value; }\n",
        "fnc read(borrow Thing thing) -> int { return *thing.value; }\n",
        "fnc read(Pointer<int> thing) -> int { return 1; }\n",
        "fnc read(RawPointer<int> thing) -> int { return 1; }\n",
        "fnc read(borrow mut Thing thing) -> int { return rt borrow mut thing.value; }\n",
        "fnc read(Thing thing) -> int { return rt ref thing; }\n",
        "fnc read(Thing thing) -> int { return rt deref thing; }\n",
        "fnc read(Thing thing) -> int { return rt unref thing; }\n",
        "fnc read(Thing thing) -> int { return rt raw thing; }\n",
        "define class Cursor as pub next(self &mut Cursor)() -> int { return 1; } end\n",
    ] {
        assert!(format_source(source).is_err(), "{source}");
    }
}

#[test]
fn binary_ampersand_and_multiplication_remain_formatable() {
    let source = "fnc calc(int a, int b): int { return (a & b) * 2; }\n";
    assert!(format_source(source).is_ok());
}

#[test]
fn pointer_guard_uses_token_boundaries() {
    for source in [
        "fnc rtrefactor(int value): int { return value; }\n",
        "define class SomePointer as\nend\n",
        "define class RawPointerAlias as\nend\n",
        "fnc calc(int mutator, int x): int { return x & mutator; }\n",
    ] {
        assert!(format_source(source).is_ok(), "{source}");
    }
}

#[test]
fn pointer_guard_rejects_more_expression_and_type_starts() {
    for source in [
        "fnc bad(): &Thing { return value; }\n",
        "fnc f() -> & mut Thing :: return value; end\n",
        "fnc i() -> &mut Thing :: return value; end\n",
        "fnc bad(): int { return [&value][0]; }\n",
        "fnc bad(): int { return [*value][0]; }\n",
        "fnc bad(): int { return condition ? 1 : &value; }\n",
        "fnc bad(): int { return condition ? 1 : *value; }\n",
        "define class Cursor as pub next(self & mut Cursor)() -> int { return 1; } end\n",
    ] {
        assert!(format_source(source).is_err(), "{source}");
    }
}

#[test]
fn module_owned_class_is_indented_as_a_module_member() {
    let src = r#"define module Accounts as
define class Account as
pub let int balance = 100;

pub read() => int {
return self.balance;
}
end

pub fnc seed() => int {
return 100;
}
end
"#;

    let expected = r#"define module Accounts as
  define class Account as
    pub let int balance = 100;

    pub read() -> int {
      return self.balance;
    }
  end

  pub fnc seed() -> int {
    return 100;
  }
end
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn tuple_and_fixed_sequence_metadata_remain_stable() {
    let src = r#"fnc build() => void {
val pair: Tuple<align=64, size=2>[int, string] = tuple(1, "one");
val fixed: FixedArray<size=4, align=32>[4 of int] = [1, 2, 3, 4];
return;
}
"#;

    let expected = r#"fnc build() -> void {
  val pair: Tuple<align=64, size=2>[int, string] = tuple(1, "one");
  val fixed: FixedArray<size=4, align=32>[4 of int] = [1, 2, 3, 4];
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn channel_select_and_nb_callback_spellings_remain_stable() {
    let src = r#"fnc poll() => void {
val choice = nb select first from cases;
val read = try readch input;
val wrote = try writech output, 42;
nb cb writech output, 42 || -> {
stdio.stdout.write("sent");
};
return;
}
"#;

    let expected = r#"fnc poll() -> void {
  val choice = nb select first from cases;
  val read = try readch input;
  val wrote = try writech output, 42;
  nb cb writech output, 42 || -> {
    stdio.stdout.write("sent");
  };
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn unused_import_selector_spellings_are_commented_not_deleted() {
    let src = r#"// oresfmt: pure-import
import * as util from "../util";
// oresfmt: pure-import
import module Math from "../math";
// oresfmt: pure-import
import actor Worker from "../worker";
// oresfmt: pure-import
import class Box from "../box";
// oresfmt: pure-import
import types X, Y, Z from "../types";
// oresfmt: pure-import
import types (A, B, C) from "../more-types";

define module App as
pub fnc main() => void {
return;
}
end
"#;

    let expected = r#"// oresfmt: pure-import
// import * as util from "../util";
// oresfmt: pure-import
// import module Math from "../math";
// oresfmt: pure-import
// import actor Worker from "../worker";
// oresfmt: pure-import
// import class Box from "../box";
// oresfmt: pure-import
// import types X, Y, Z from "../types";
// oresfmt: pure-import
// import types (A, B, C) from "../more-types";

define module App as
  pub fnc main() -> void {
    return;
  }
end
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn class_contract_headers_and_nested_contracts_indent_without_confusion() {
    let src = r#"define class Child extends Parent<int>, Audited implements Named, Serializable as
pub val String name;

pub render() => String {
return self.name;
}


define interface LocalShape
fnc project(String value) -> String;
end

define trait Retryable
fnc retry(): bool;
end
end
"#;
    let expected = r#"define class Child extends Parent<int>, Audited implements Named, Serializable as
  pub val String name;

  pub render() -> String {
    return self.name;
  }

  define interface LocalShape
    fnc project(String value) => String;
  end

  define trait Retryable
    fnc retry() => bool;
  end
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn comments_and_strings_do_not_change_block_depth() {
    let src = r#"fnc text() => String {
// } should not dedent
return "{still text}";
}

fnc template() => String {
return `}`;
}
"#;
    let expected = r#"fnc text() -> String {
  // } should not dedent
  return "{still text}";
}


fnc template() -> String {
  return `}`;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn formats_if_fi_and_actor_bodies() {
    let src = r#"shared actor Cleaner {
pub fnc compact() => void {
if ready; do
actor.gc();
else
return;
fi
}
}
"#;
    let expected = r#"shared actor Cleaner {
  pub compact(): void {
    if ready; then
      actor.gc();
    else
      return;
    fi
  }
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn formatter_is_idempotent_and_normalizes_crlf() {
    let src = "pub routine main() => void {\r\n  return;  \r\n}\r\n";
    let once = format_source(src).unwrap();
    let twice = format_source(&once).unwrap();
    assert_eq!(once, twice);
    assert_eq!(
        once,
        "pub routine main() -> void {
  return;
}
"
    );
    assert!(is_formatted(&once).unwrap());
}

#[test]
fn rejects_unbalanced_blocks() {
    let err = format_source(
        "fnc nope() -> void {
",
    )
    .unwrap_err();
    assert!(err.message().contains("unterminated"));
}

#[test]
fn local_interfaces_inside_functions_override_outer_callable_context() {
    let src = r#"fnc outer() => void {
define interface Local
fnc run(): int;
end
return;
}
"#;
    let expected = r#"fnc outer() -> void {
  define interface Local
    fnc run() => int;
  end
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn braced_interfaces_use_signature_arrows() {
    let src = r#"pub interface Shape {
fnc area() -> f64;
}
"#;
    let expected = r#"pub interface Shape {
  fnc area() => f64;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn annotations_stay_attached_to_the_next_function() {
    let src = r#"fnc one() => int {
return 1;
}
@Ret<self>
fnc two() => int {
return 2;
}
"#;
    let expected = r#"fnc one() -> int {
  return 1;
}


@Ret<self>
fnc two() -> int {
  return 2;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn canonicalizes_parenthesized_callable_return_types() {
    let src = r#"fnc make_callback(): (() => int) {
return || -> {
return 1;
};
}

define interface Factory
fnc make_callback() -> (() => int);
end
"#;
    let expected = r#"fnc make_callback() -> (() => int) {
  return || -> {
    return 1;
  };
}

define interface Factory
  fnc make_callback() => (() => int);
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn callable_arrow_ignores_braces_inside_default_strings() {
    let src = r#"fnc render(String marker = "{") => String {
return marker;
}
"#;
    let expected = r#"fnc render(String marker = "{") -> String {
  return marker;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn rejects_multiline_templates_instead_of_mutating_literal_bytes() {
    let src = "fnc template() => String {\n  return `line one\n    line two`;\n}\n";
    let err = format_source(src).unwrap_err();
    assert!(
        err.message()
            .contains("multiline string/template literals are not formatted yet")
    );
}

#[test]
fn validates_if_fi_and_do_done_terminators() {
    let valid = r#"fnc work() => void {
if ready; do
while busy; do
tick();
done
else
return;
fi
}
"#;
    let formatted = format_source(valid).unwrap();
    assert!(formatted.contains("  if ready; then"));
    assert!(formatted.contains("    while busy; do"));
    assert!(formatted.contains("  fi"));

    let wrong = "if ready; do\nreturn;\ndone\n";
    let err = format_source(wrong).unwrap_err();
    assert!(err.message().contains("`done` where `fi` was expected"));

    let orphan = "else\nreturn;\n";
    let err = format_source(orphan).unwrap_err();
    assert!(
        err.message()
            .contains("without a matching `if ... then` block")
    );
}

#[test]
fn rejects_unterminated_keyword_blocks() {
    let err = format_source("if ready; do\nreturn;\n").unwrap_err();
    assert!(err.message().contains("expected `fi`"));

    let err = format_source("while ready; do\ntick();\n").unwrap_err();
    assert!(err.message().contains("expected `done`"));
}

#[test]
fn canonicalizes_route_guard_do_to_then() {
    let src = r#"fnc validate(int handler_id, Pattern pattern) => Result {
if handler_id < 0; do
return Err("handler_id must be non-negative");
fi
if !pattern.is_valid(); do
return Err("invalid route pattern");
fi
if pattern.size() > self.max_segments_value; do
return Err("route pattern exceeds max_segments");
fi
}
"#;

    let expected = r#"fnc validate(int handler_id, Pattern pattern) -> Result {
  if handler_id < 0; then
    return Err("handler_id must be non-negative");
  fi
  if !pattern.is_valid(); then
    return Err("invalid route pattern");
  fi
  if pattern.size() > self.max_segments_value; then
    return Err("route pattern exceeds max_segments");
  fi
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn multiline_block_comments_cannot_change_structure() {
    let src = r#"define class Box as
/*
end
if fake; do
}
*/
pub get() => int {
return 1;
}
end
"#;
    let expected = r#"define class Box as
  /*
  end
  if fake; do
  }
  */
  pub get() -> int {
    return 1;
  }
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn inline_block_comments_do_not_confuse_callable_boundaries() {
    let src = r#"/* fake ( { end */ fnc real(String marker = "{") => String {
return marker;
}
"#;
    let expected = r#"/* fake ( { end */ fnc real(String marker = "{") -> String {
  return marker;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn block_comments_act_as_lexical_separators() {
    let src = r#"define/* comment */class Box as
pub get() => int {
return 1;
}
end
"#;
    let expected = r#"define/* comment */class Box as
  pub get() -> int {
    return 1;
  }
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn canonicalizes_conditional_compatibility_spellings() {
    let src = r#"fnc choose(int value) => int {
if value < 0 do
return -1;
elseif value == 0; do
return 0;
else if value == 1 then
return 1;
else
return 2;
fi
}
"#;
    let expected = r#"fnc choose(int value) -> int {
  if value < 0 then
    return -1;
  elif value == 0; then
    return 0;
  elif value == 1 then
    return 1;
  else
    return 2;
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn conditional_rewrites_ignore_strings_and_comments() {
    let src = r#"fnc text() => String {
// elseif fake; do
if ready; do
return "else if nope do";
else
return "then";
fi
}
"#;
    let expected = r#"fnc text() -> String {
  // elseif fake; do
  if ready; then
    return "else if nope do";
  else
    return "then";
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn canonicalizes_braced_branch_aliases_without_inventing_then() {
    let src = r#"if first {
work();
}
elseif second {
other();
}
else if third {
third_work();
}
else {
fallback();
}
fi
"#;
    let expected = r#"if first {
  work();
}
elif second {
  other();
}
elif third {
  third_work();
}
else {
  fallback();
}
fi
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn nested_braced_if_chain_keeps_enclosing_callable_indent() {
    let src = r#"fnc choose(int value) => int {
if value < 0 {
return -1;
}
else if value == 0 {
return 0;
}
else {
return 1;
}
fi
}
"#;
    let expected = r#"fnc choose(int value) -> int {
  if value < 0 {
    return -1;
  }
  elif value == 0 {
    return 0;
  }
  else {
    return 1;
  }
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn select_cases_use_spaces_and_braced_bodies_in_every_wait_mode() {
    for mode in ["", "nb ", "try "] {
        let source = format!(
            "actor fnc relay() -> void {{\n\t{mode}select first {{\n\tcase readch inbox: val value\n\t\tnb writech replies, value * 10;\n\tcase writech replies, 0:\n\t\treturn;\n\tdefault:\n\t\treturn;\n\t}}\n}}\n"
        );
        let expected = format!(
            "actor fnc relay() -> void {{\n  {mode}select first {{\n    when readch inbox: val value {{\n      nb writech replies, value * 10;\n    }}\n    when writech replies, 0: {{\n      return;\n    }}\n    default: {{\n      return;\n    }}\n  }}\n}}\n"
        );
        let formatted = format_source(&source).unwrap();
        assert_eq!(formatted, expected);
        assert!(!formatted.contains('\t'));
        assert!(is_formatted(&formatted).unwrap());
    }
}

#[test]
fn select_case_is_accepted_but_canonicalized_to_when() {
    let source = "do nb select { case readch messages: val msg { stdio.println(msg); } }\n";
    let expected =
        "do nb select {\n  when readch messages: val msg {\n    stdio.println(msg);\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(expected).unwrap());

    let already_canonical =
        "do nb select {\n  when readch messages: val msg {\n    stdio.println(msg);\n  }\n}\n";
    assert_eq!(format_source(already_canonical).unwrap(), already_canonical);
}

#[test]
fn compact_and_nested_select_arms_expand_without_changing_literals_or_comments() {
    let source = r#"select { case readch inbox: val value { nb select {
case writech replies, value: // sent
stdio.println("case default: { } λ");
default: /* skipped */
return;
} } default: {} }
"#;
    let expected = r#"select {
  when readch inbox: val value {
    nb select {
      when writech replies, value: { // sent
        stdio.println("case default: { } λ");
      }
      default: { /* skipped */
        return;
      }
    }
  }
  default: {
  }
}
"#;
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(format_source(&formatted).unwrap(), expected);
}

#[test]
fn select_migration_handles_inline_legacy_arms_nested_expressions_and_empty_arms() {
    let source = "select random { case writech output, make(\"x:y\", [1, 2]): send(); case readch input: const x; default: }\n";
    let expected = "select random {\n  when writech output, make(\"x:y\", [1, 2]): {\n    send();\n  }\n  when readch input: const x {\n  }\n  default: {\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn streaming_channel_writes_format_in_for_and_for_await_loops() {
    let source = "pub async routine pump() -> void {\n\tfor const value of [1, 2, 3] do\n\t\twritech output, value;\n\tdone\n\tfor await const value of values() {\n\t\tawait nb writech output, value;\n\t}\n}\n";
    let expected = "pub async routine pump() -> void {\n  for const value of [1, 2, 3] do\n    writech output, value;\n  done\n  for await const value of values() {\n    await nb writech output, value;\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn malformed_static_select_fails_closed() {
    for source in [
        "select { case readch input val x return; }",
        "select { case nope: {} }",
        "nb select { case readch input: val x {}",
    ] {
        assert!(format_source(source).is_err(), "{source}");
    }
}

#[test]
fn select_inline_block_comments_do_not_hide_arm_or_body_line_breaks() {
    let source = "select { /* choose */ case readch inbox: val value { /* body */ send(value); } /* next */ default: { /* empty */ } }\n";
    let expected = "select { /* choose */\n  when readch inbox: val value { /* body */\n    send(value);\n  } /* next */\n  default: { /* empty */\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn select_migration_preserves_nested_legacy_select_and_keyword_loop_bodies() {
    let source = "select {\ncase readch input: val x\nfor const value of [x] do\nnb select {\ncase writech output, value:\nreturn;\n}\ndone\ndefault:\n}\n";
    let expected = "select {\n  when readch input: val x {\n    for const value of [x] do\n      nb select {\n        when writech output, value: {\n          return;\n        }\n      }\n    done\n  }\n  default: {\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn select_header_separator_ignores_ternary_colons_and_optional_casts() {
    let source = "select { case readch ready ? inbox : other: val x {} case writech output, ready ? 1 : otherReady ? 2 : 3: {} default: {} }\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("when readch ready ? inbox : other: val x {\n"));
    assert!(formatted.contains("when writech output, ready ? 1 : otherReady ? 2 : 3: {\n"));
    assert!(is_formatted(&formatted).unwrap());
    let cast = format_source("select { case writech output, value as? int: {} }\n").unwrap();
    assert!(cast.contains("when writech output, value as? int: {\n"));
    assert!(is_formatted(&cast).unwrap());
}

#[test]
fn canonicalizes_route_guard_conditionals_to_then_and_elif() {
    let src = r#"fnc validate(int handler_id, Pattern pattern) -> Result {
if handler_id < 0; do
return Err("handler_id must be non-negative");
elseif !pattern.is_valid(); do
return Err("invalid route pattern");
else if pattern.size() > self.max_segments_value then
return Err("route pattern exceeds max_segments");
else
return Ok();
fi
}
"#;

    let expected = r#"fnc validate(int handler_id, Pattern pattern) -> Result {
  if handler_id < 0; then
    return Err("handler_id must be non-negative");
  elif !pattern.is_valid(); then
    return Err("invalid route pattern");
  elif pattern.size() > self.max_segments_value then
    return Err("route pattern exceeds max_segments");
  else
    return Ok();
  fi
}
"#;

    let formatted = format_source(src).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn canonicalizes_bare_for_forever_aliases_to_loop() {
    let source = r#"fnc spin() => void {
for {
tick();
break;
}
for do
tick();
break;
done
for (let i = 0; i < 2; i = i + 1) {
tick();
}
for const item of values do
tick(item);
done
}
"#;
    let expected = r#"fnc spin() -> void {
  loop {
    tick();
    break;
  }
  loop do
    tick();
    break;
  done
  for (let i = 0; i < 2; i = i + 1) {
    tick();
  }
  for const item of values do
    tick(item);
  done
}
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn formats_pre_test_and_post_test_while_forms() {
    let source = r#"fnc loops() => void {
while ready do
tick();
done
while ready {
tick();
}
do {
tick();
} while (ready);
block {
val local = 1;
stdio.stdout.write(local);
}
}
"#;
    let expected = r#"fnc loops() -> void {
  while ready do
    tick();
  done
  while ready {
    tick();
  }
  do {
    tick();
  } while (ready);
  block {
    val local = 1;
    stdio.stdout.write(local);
  }
}
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn bare_for_rewrite_is_comment_aware_and_does_not_touch_real_for_headers() {
    let source = r#"for /* forever */ {
break;
}
for (;;) {
break;
}
for await const item of stream() {
consume(item);
}
"#;
    let expected = r#"loop /* forever */ {
  break;
}
for (;;) {
  break;
}
for await const item of stream() {
  consume(item);
}
"#;
    assert_eq!(format_source(source).unwrap(), expected);
}

#[test]
fn preserves_orthogonal_binding_and_select_case_qualifiers() {
    // Compiler contract: const = fixed/read-only, val = const mut,
    // let = rebindable/read-only, let mut = rebindable/mutable.
    // The formatter must never drop or reorder mutability qualifiers.
    let source = r#"fnc example(): void {
const item = make();
const mut editable = make();
val owned = make();
let replaceable = make();
let mut flexible = make();
select {
case readch input: const job {
consume(job);
}
case readch output: val job {
job.foo = 1;
}
}
return;
}
"#;
    let output = format_source(source).unwrap();
    assert!(output.contains("const item = make();"));
    assert!(output.contains("const mut editable = make();"));
    assert!(output.contains("val owned = make();"));
    assert!(output.contains("let replaceable = make();"));
    assert!(output.contains("let mut flexible = make();"));
    assert!(output.contains("when readch input: const job {"));
    assert!(output.contains("when readch output: val job {"));
}

#[test]
fn canonicalizes_name_first_parameter_compatibility_to_type_first() {
    let source = r#"fnc run(job: Job, options: Options): Result {
return dispatch(job: job, options: options);
}
"#;
    let expected = r#"fnc run(Job job, Options options) -> Result {
  return dispatch(job: job, options: options);
}
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn canonicalizes_name_first_parameters_in_methods_and_interface_signatures() {
    let source = r#"define class Worker as
pub run(job: Job): Result {
return process(job);
}
end

define interface Runner
run(job: Job): Result;
end
"#;
    let expected = r#"define class Worker as
  pub run(Job job) -> Result {
    return process(job);
  }
end

define interface Runner
  run(Job job) => Result;
end
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn parameter_order_rewrite_understands_nested_structural_types() {
    let source = r#"fnc inspect(value: {id: string, active: bool}): string {
return value.id;
}
"#;
    let expected = r#"fnc inspect({id: string, active: bool} value) -> string {
  return value.id;
}
"#;
    assert_eq!(format_source(source).unwrap(), expected);
}

#[test]
fn legacy_instance_fnc_and_method_use_canonical_instance_spelling() {
    let input = r#"define actor ConnectionActor as
final rooms: Map<RoomId, ActorGroup>

suspend fnc join_room(
group: ActorGroup,
permit: ActorGroupJoinCapability
) {
group.join(permit)
}
fnc leave_room(group: ActorGroup) {
group.leave()
}
suspend method explicit_join(group: ActorGroup) {
group.join()
}
static fnc static_send(group: ActorGroup) {
group.leave()
}
end
define class Client as
fnc legacy(): int { return 1; }
method explicit(): int { return 2; }
static fnc helper(): int { return 3; }
end
fnc standalone(): int { return 4; }
"#;
    let out = format_source(input).unwrap();
    assert!(out.contains("suspend join_room("), "{out}");
    assert!(out.contains("  leave_room(ActorGroup group) {"), "{out}");
    assert!(out.contains("suspend mtd explicit_join("), "{out}");
    assert!(out.contains("static fnc static_send("), "{out}");
    assert!(out.contains("  legacy() -> int {"), "{out}");
    assert!(out.contains("  mtd explicit() -> int {"), "{out}");
    assert!(out.contains("  static fnc helper() -> int {"), "{out}");
    assert!(out.contains("fnc standalone() -> int {"), "{out}");
    assert!(!out.contains("fnc join_room"), "{out}");
    assert!(!out.contains("fnc leave_room"), "{out}");
    assert_eq!(format_source(&out).unwrap(), out);
}

#[test]
fn never_rewrites_nested_or_string_or_comment_fnc() {
    let src = r#"define actor User as
fnc call(): String {
  // fnc marker() is a comment
  val title = "fnc name()";
  return title;
}
static fnc direct(): int { return 42; }
end
"#;
    let out = format_source(src).unwrap();
    assert!(out.contains("call(): String {"));
    assert!(out.contains("// fnc marker() is a comment"));
    assert!(out.contains("\"fnc name()\""));
    assert!(out.contains("static fnc direct()"));
    assert_eq!(format_source(&out).unwrap(), out);
}

#[test]
fn modifier_permutations_format_in_one_pass_without_changing_static_dispatch() {
    let src = r#"define class Client as
fnc pub legacy(): int { return 1; }
fnc suspend pub deferred(): int { return 2; }
pub fnc async compute(): int { return 3; }
fnc static pub reusable(): int { return 4; }
fnc method_name(): int { return 5; }
method method(): int { return 6; }
end
"#;
    let once = format_source(src).unwrap();
    assert!(once.contains("  pub legacy() -> int {"), "{once}");
    assert!(once.contains("  pub suspend deferred() -> int {"), "{once}");
    assert!(once.contains("  pub async compute() -> int {"), "{once}");
    assert!(
        once.contains("  pub static fnc reusable() -> int {"),
        "{once}"
    );
    assert!(once.contains("  method_name() -> int {"), "{once}");
    assert!(once.contains("  mtd method() -> int {"), "{once}");
    assert_eq!(once, format_source(&once).unwrap());
}

#[test]
fn method_normalization_never_mutates_comments_literals_or_static_fnc() {
    let src = r#"define class Client as
// fnc ignored(): int
static fnc keep(): int { return 1; }
fnc ordinary(): String {
  return "fnc literal";
}
end
"#;
    let formatted = format_source(src).unwrap();
    assert!(formatted.contains("// fnc ignored(): int"));
    assert!(formatted.contains("  static fnc keep() -> int"));
    assert!(formatted.contains("  ordinary() -> String"));
    assert!(formatted.contains("\"fnc literal\""));
    assert_eq!(formatted, format_source(&formatted).unwrap());
}

#[test]
fn bare_keyword_actor_end_body_is_an_instance_method_scope() {
    let src = r#"shared actor Worker as
fnc handle(): int { return 1; }
method explicit(): int { return 2; }
static fnc helper(): int { return 3; }
end
"#;
    let out = format_source(src).unwrap();
    assert!(out.contains("  handle(): int {"), "{out}");
    assert!(out.contains("  mtd explicit(): int {"), "{out}");
    assert!(out.contains("  static fnc helper(): int {"), "{out}");
    assert_eq!(out, format_source(&out).unwrap());
}

#[test]
fn multiline_connection_actor_method_params_are_indented_and_idempotent() {
    let source = r#"define actor ConnectionActor as
final rooms: Map<RoomId, ActorGroup>

suspend fnc join_room(
group: ActorGroup,
permit: ActorGroupJoinCapability
) {
group.join(permit)
}
fnc leave_room(group: ActorGroup) {
group.leave()
}
end
"#;
    let expected = r#"define actor ConnectionActor as
  final rooms: Map<RoomId, ActorGroup>

  suspend join_room(
    group: ActorGroup,
    permit: ActorGroupJoinCapability
  ) {
    group.join(permit)
  }
  leave_room(ActorGroup group) {
    group.leave()
  }
end
"#;
    let out = format_source(source).unwrap();
    assert_eq!(out, expected);
    assert_eq!(format_source(&out).unwrap(), out);
}

#[test]
fn define_shared_actor_is_also_a_method_container() {
    let source = r#"define shared actor Worker as
fnc handle(): int { return 7; }
static fnc utility(): int { return 9; }
end
"#;
    let result = format_source(source).unwrap();
    assert!(result.contains("  handle(): int {"), "{result}");
    assert!(result.contains("  static fnc utility(): int {"), "{result}");
    assert_eq!(format_source(&result).unwrap(), result);
}
#[test]
fn braced_switch_fallback_first_and_extra_separators_have_canonical_layout() {
    let source = r#"
pub fnc use_switch(string v) : string {
switch (v) {
_ {
return "star";
},
case "foo", "car", "mar" {
return "bar";
};
}
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("switch (v) {"), "{formatted}");
    assert!(
        formatted.contains("case \"foo\", \"car\", \"mar\" {"),
        "{formatted}"
    );
    assert!(!formatted.contains("},\n"), "{formatted}");
    assert!(!formatted.contains("};\n"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn braced_and_end_delimited_match_variants_format_idempotently() {
    let braced = r#"define actor Counter as
receive(ActorMail<MessagePatterns> mail): void {
match mail.value over {
on is MessagePatterns.Reset as m {
handle(m.reset);
},
on MessagePatterns.Tuple as (m1,m2) {
handle_pair(m1,m2);
};
on _ { return; }
}
}
end
"#;
    let canonical = format_source(braced).unwrap();
    assert!(
        canonical.contains("on is MessagePatterns.Reset as m {"),
        "{canonical}"
    );
    assert!(
        canonical.contains("on MessagePatterns.Tuple as (m1,m2) {"),
        "{canonical}"
    );
    assert!(!canonical.contains("},\n"), "{canonical}");
    assert_eq!(format_source(&canonical).unwrap(), canonical);

    let end_delimited = r#"fnc f(Option<int> v): int {
match v over
on Some(value) {
return value;
}
on _ {
return 0;
}
end
}
"#;
    let output = format_source(end_delimited).unwrap();
    assert!(output.contains("match v over"), "{output}");
    assert!(output.contains("  end\n"), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
}

#[test]
fn switch_default_and_match_wildcard_remain_distinct() {
    let source = r#"fnc choose(string v): string {
  switch v {
    _ { return "star"; }
    case "foo", "car", "mar" { return "bar"; }
  }
}
fnc check(int n): int {
  match n over {
    on _ as m { return m; }
    on 7 { return 42; }
  }
}
fnc other(int n): int {
  match n over {
    on default { return 0; }
  }
}
"#;
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("default {\n"), "{formatted}");
    assert!(formatted.contains("on _ as m {\n"), "{formatted}");
    assert!(!formatted.contains("on default"), "{formatted}");
    assert!(
        formatted.contains("case \"foo\", \"car\", \"mar\" {"),
        "{formatted}"
    );
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn legacy_end_delimited_fallbacks_are_formatted_as_default() {
    let source = r#"define module M as
fnc choose(int n): int {
switch n
case 7 -> {
return 42;
}
_ -> {
return 0;
}
end
}
fnc inspect(int n): int {
match n over
on 7 -> {
return 42;
}
on _ as other -> {
return other;
}
end
}
end
"#;
    let once = format_source(source).unwrap();
    assert!(once.contains("default -> {"), "{once}");
    assert!(once.contains("on _ as other -> {"), "{once}");
    assert!(!once.contains("\n  _ ->"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn legacy_default_normalization_never_rewrites_multiline_comment_content() {
    let src = r#"fnc inspect(int n): int {
match n over
/*
on _ -> { phantom(); }
*/
on 7 -> {
return 7;
}
on _ -> {
return 0;
}
end
}
"#;
    let formatted = format_source(src).unwrap();
    assert!(formatted.contains("on _ -> { phantom(); }"), "{formatted}");
    assert!(formatted.contains("on _ -> {"), "{formatted}");
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn typed_match_return_contracts_preserve_named_parameter_and_result_mode() {
    let source = r#"fnc choose(int n): bool {
match<ret=bool> n over {
on 7 { return true; }
on _ as remaining { return false; }
}
}
fnc procedural(int n): bool {
do match<ret=bool> n over {
on _ as remaining { return false; }
on 7 { return true; }
}
}
"#;
    let once = format_source(source).unwrap();
    assert!(once.contains("match<ret=bool> n over {"), "{once}");
    assert!(once.contains("do match<ret=bool> n over {"), "{once}");
    assert!(once.contains("on _ as remaining {"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn generic_defaults_and_named_type_arguments_are_semantically_preserved() {
    let source = r#"fnc choose<A, B=int, C=B, D=Array<C>, E=string>(A input): int {
  return 7;
}
fnc use(): int {
  val int first = choose<A=int, D=Array<string>>(7);
  val int second = choose<E=bool, A=int>(7);
  val int third = choose<int, _>(7);
  return first + second + third;
}
"#;
    let once = format_source(source).unwrap();
    assert!(
        once.contains("choose<A, B=int, C=B, D=Array<C>, E=string>"),
        "{once}"
    );
    assert!(once.contains("choose<A=int, D=Array<string>>(7)"), "{once}");
    assert!(once.contains("choose<E=bool, A=int>(7)"), "{once}");
    assert!(once.contains("choose<int, _>(7)"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
    assert!(is_formatted(&once).unwrap());
}

#[test]
fn generic_inference_hole_and_comparison_like_tokens_do_not_mutate() {
    let source = r#"fnc choose<T=int>(): int {
  return 7;
}
fnc use(): int {
  val int first = choose<T=_>();
  val int second = choose<Array<_>>();
  val int third = choose<Option<Array<_>>>();
  return first + second + third;
}
"#;
    let once = format_source(source).unwrap();
    for preserved in [
        "choose<T=_>()",
        "choose<Array<_>>()",
        "choose<Option<Array<_>>>()",
    ] {
        assert!(once.contains(preserved), "{preserved}: {once}");
    }
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn typed_lambda_and_explicit_tuple_record_forms_are_stable() {
    let source = r#"type Mapper = (int) => int;
fnc demo(): void {
const mapper: Mapper = |int n| -> (n * 3) as int;
const declared = |int n|: int -> n * 3;
const pair = tuple(4, "four");
const singleton = tuple(7) as tuple[int];
const point = struct{x: int, y: int}{x: 3, y: 5};
const just_x = point as {x: int};
return;
}
"#;
    let formatted = format_source(source).expect("valid typed lambda and aggregate syntax");
    for fragment in [
        "|int n| -> (n * 3) as int",
        "|int n|: int -> n * 3",
        "tuple(4, \"four\")",
        "tuple(7) as tuple[int]",
        "struct{x: int, y: int}{x: 3, y: 5}",
        "point as {x: int}",
    ] {
        assert!(
            formatted.contains(fragment),
            "formatter lost the required syntax {fragment:?}: {formatted}"
        );
    }
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn transparent_type_aliases_preserve_generic_targets_and_canonicalize_visibility() {
    let source = r#"pub define type alias Pair<K, V> as Entry<K, V>;
define type alias Two<A, B> as Tuple[A, B];
type Old = Entry<int, string>;
type alias = int;
define type alias = string;
"#;
    let formatted = format_source(source).expect("format aliases");
    assert!(
        formatted.contains("define pub type alias Pair<K, V> as Entry<K, V>;"),
        "{formatted}"
    );
    assert!(
        formatted.contains("define type alias Two<A, B> as Tuple[A, B];"),
        "{formatted}"
    );
    assert!(
        formatted.contains("type Old = Entry<int, string>;"),
        "{formatted}"
    );
    assert!(formatted.contains("type alias = int;"), "{formatted}");
    assert!(
        formatted.contains("define type alias = string;"),
        "{formatted}"
    );
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn comments_and_literals_cannot_reorder_type_alias_words() {
    let source = r#"// pub define type alias Fake as Entry;
const string example = "pub define type alias Fake as Entry";
pub define /* comment */ type alias Real as Entry;
"#;
    let formatted = format_source(source).expect("format alias comments");
    assert!(formatted.contains("// pub define type alias Fake as Entry;"));
    assert!(formatted.contains("\"pub define type alias Fake as Entry\""));
    assert!(formatted.contains("pub define /* comment */ type alias Real as Entry;"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn canonical_simple_module_headers_use_as() {
    let legacy = "define module products\n  define pub type alias Pair<K,V> as Tuple[K,V];\nend\n";
    let expected =
        "define module products as\n  define pub type alias Pair<K,V> as Tuple[K,V];\nend\n";
    assert_eq!(format_source(legacy).unwrap(), expected);
    assert_eq!(format_source(expected).unwrap(), expected);
}

#[test]
fn module_header_comments_and_nonheaders_are_not_rewritten() {
    let source = "define module core // comment\nend\n";
    let result = format_source(source).unwrap();
    assert!(
        result.starts_with("define module core as // comment\n"),
        "{result}"
    );
    assert_eq!(format_source(&result).unwrap(), result);

    let already = "define module core as\nend\n";
    assert_eq!(format_source(already).unwrap(), already);

    // Never manufacture an `as` in strings or unrelated expressions.
    let quote = "const string example = \"define module core\";\n";
    assert_eq!(format_source(quote).unwrap(), quote);
}

#[test]
fn colon_initializers_are_canonical_only_for_direct_type_first_class_fields() {
    let src = r#"define class Foo as
final int foo = 5
string bar = "100"
Map<string, Array<int>> cache = factory()
bar: string = "typed"
fnc update(): void {
  foo = 6;
  const value = struct{bar: "literal = stays"};
  return;
}
end
"#;
    let expected = r#"define class Foo as
  final int foo: 5
  string bar: "100"
  Map<string, Array<int>> cache: factory()
  bar: string = "typed"
  update() -> void {
    foo = 6;
    const value = struct{bar: "literal = stays"};
    return;
  }
end
"#;
    let actual = format_source(src).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(format_source(&actual).unwrap(), actual);
}

#[test]
fn colon_initializer_keeps_comments_strings_and_function_assignments_untouched() {
    let src = r#"define class C as
  int one = 3 // = inside a comment
  string label = "a=b:c"
  int two: 4
  change(): void {
    one = 9;
    const v = {"key": "="};
  }
end
"#;
    let actual = format_source(src).unwrap();
    assert!(actual.contains("int one: 3 // = inside a comment"));
    assert!(actual.contains("string label: \"a=b:c\""));
    assert!(actual.contains("int two: 4"));
    assert!(actual.contains("one = 9;"));
    assert!(actual.contains("\"key\": \"=\""));
    assert_eq!(format_source(&actual).unwrap(), actual);
}

#[test]
fn colon_normalization_preserves_legacy_binding_kind_fields_and_nested_code() {
    let src = r#"define class Box as
  pub let int balance = 100;
  val string label = "fixed";
  const int limit = 9;
  let mut Array<int> items = new Array<int>();
  final int identifier = 42
  string name = "box"
  change(): void {
    let int n = 2;
    balance = 3;
  }
end
"#;
    let got = format_source(src).unwrap();
    // Explicit binding keywords are semantic and Java's parseField only
    // accepts `=` after them, so these lines are left exactly as written.
    for unchanged in [
        "pub let int balance = 100;",
        "val string label = \"fixed\";",
        "const int limit = 9;",
        "let mut Array<int> items = new Array<int>();",
        "let int n = 2;",
        "balance = 3;",
    ] {
        assert!(
            got.contains(unchanged),
            "rewrote an unrelated construct: {unchanged} in {got}"
        );
    }
    assert!(got.contains("final int identifier: 42"));
    assert!(got.contains("string name: \"box\""));
    assert_eq!(got, format_source(&got).unwrap());
}

#[test]
fn multiline_method_parameter_defaults_are_not_field_initializers() {
    let src = r#"define class Foo as
  int field = 42
  compute(
    int x = 5,
    string label = "ok"
  ): int {
    return x;
  }
end
"#;
    let got = format_source(src).unwrap();
    assert!(got.contains("int field: 42"), "{got}");
    assert!(got.contains("int x = 5"), "{got}");
    assert!(got.contains("string label = \"ok\""), "{got}");
    assert!(!got.contains("int x: 5"), "{got}");
    assert!(!got.contains("string label: \"ok\""), "{got}");
    assert_eq!(got, format_source(&got).unwrap());
}

#[test]
fn quotes_and_template_interpolations_remain_byte_for_byte_intact() {
    let src = r#"pub fnc main(): void {
const foo1 = "abc";
const foo2 = "'abc'";
const foo3 = '"abc"';
const foo4 = `'"abc"'`;
const bar = "Mars";
const star = "bright";
const greeting = `${bar} is ${star}`;
const escaped = `escaped \${notInterpolated} and \``;
return;
}
"#;
    let once = format_source(src).unwrap();
    for fragment in [
        r#""abc""#,
        r#""'abc'""#,
        r#"'"abc"'"#,
        r#"`'"abc"'`"#,
        r#"`${bar} is ${star}`"#,
        r#"`escaped \${notInterpolated} and \``"#,
    ] {
        assert!(
            once.contains(fragment),
            "changed string literal: {fragment}; got {once}"
        );
    }
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn typed_const_fnc_template_body_keeps_lambda_and_interpolation_bytes() {
    let source = r#"const fnc foo = |string bar, string star| -> {
return `${bar} is ${star}`;
};
"#;
    let once = format_source(source).unwrap();
    assert!(
        once.contains("const fnc foo = |string bar, string star| -> {"),
        "{once}"
    );
    assert!(once.contains("return `${bar} is ${star}`;"), "{once}");
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn template_expression_tokens_are_opaque_to_conditional_and_lambda_rewrites() {
    let source = r#"pub fnc main(): void {
const word = "hi";
const complex = `start:${word} - ${"{" + "}"} ? -> : done`;
return;
}
"#;
    let once = format_source(source).unwrap();
    assert!(
        once.contains(r#"`start:${word} - ${"{" + "}"} ? -> : done`"#),
        "{once}"
    );
    assert_eq!(format_source(&once).unwrap(), once);
}

#[test]
fn both_typed_pipe_lambda_parameter_orders_format_to_one_canonical_spelling() {
    let source = r#"const fnc first = |string bar, string star| -> {
  return bar + star;
};
const fnc second = |bar: string, star: string| -> {
  return bar + star;
};
const expr = |left: Option<string>, right: string| -> left;
"#;
    let formatted = format_source(source).expect("format both typed lambda styles");
    assert!(
        formatted.contains("const fnc first = |string bar, string star| -> {"),
        "{formatted}"
    );
    assert!(
        formatted.contains("const fnc second = |string bar, string star| -> {"),
        "{formatted}"
    );
    assert!(
        formatted.contains("const expr = |Option<string> left, string right| -> left;"),
        "{formatted}"
    );
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn typed_pipe_lambda_formatting_keeps_literals_and_other_pipes_unchanged() {
    let source = r#"const quote = "|bar: string| -> ignore";
const flag = a | b;
const fnc second = |bar: string| -> {
  return bar;
};
"#;
    let formatted = format_source(source).unwrap();
    assert!(
        formatted.contains(r#"const quote = "|bar: string| -> ignore";"#),
        "{formatted}"
    );
    assert!(formatted.contains("const flag = a | b;"), "{formatted}");
    assert!(
        formatted.contains("const fnc second = |string bar| -> {"),
        "{formatted}"
    );
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn pipe_lambda_parameter_union_must_not_be_mistaken_for_closing_delimiter() {
    let source = r#"const fnc union = |candidate: string | int| -> {
  return candidate;
};
const fnc ordinary = |string left, right: string| -> {
  return left + right;
};
"#;
    let formatted = format_source(source).expect("format typed union lambdas");
    assert!(
        formatted.contains("const fnc union = |string | int candidate| -> {"),
        "{formatted}"
    );
    assert!(
        formatted.contains("const fnc ordinary = |string left, string right| -> {"),
        "{formatted}"
    );
    assert_eq!(formatted, format_source(&formatted).unwrap());
}

#[test]
fn pipe_lambda_no_rewrite_of_arrow_in_strings_or_unrelated_pipes() {
    let src = r#"const text = "|hello: string| -> fake";
const outcome = good | bad;
const fnc actual = |name: string| -> name;
"#;
    let result = format_source(src).expect("format");
    assert!(
        result.contains(r#"const text = "|hello: string| -> fake";"#),
        "{result}"
    );
    assert!(result.contains("const outcome = good | bad;"), "{result}");
    assert!(
        result.contains("const fnc actual = |string name| -> name;"),
        "{result}"
    );
}

#[test]
fn return_suffix_identifiers_keep_binary_operators() {
    for source in [
        "fnc calc(int myreturn, int other): int { return myreturn * other; }\n",
        "fnc calc(int myreturn, int other): int { return myreturn & other; }\n",
    ] {
        let formatted = format_source(source).expect("binary operand is an identifier");
        assert_eq!(formatted, format_source(&formatted).unwrap());
    }
}

#[test]
fn class_methods_accept_bare_and_mtd_double_colon_terminators() {
    let source = r#"define class X1 as
foo(): void {}
end
define class X2 as
foo() -> void ::
end
end
define class X3 as
mtd foo(): void {}
end
define class X4 as
mtd foo() -> void ::
end
end
"#;
    let once = format_source(source).unwrap();
    assert!(once.contains("foo() -> void {"), "{once}");
    assert!(once.contains("foo() -> void ::"), "{once}");
    assert!(once.contains("mtd foo() -> void {"), "{once}");
    assert!(once.contains("mtd foo() -> void ::"), "{once}");
    assert_eq!(once, format_source(&once).unwrap());
}

#[test]
fn optional_mtd_insertion_is_idempotent_and_scope_aware() {
    let source = r#"define class C as
foo(): void {}
mtd bar() -> void ::
end
static fnc helper(): void {}
mtd(): void {}
end
define actor A as
suspend work(): void {}
end
fnc standalone(): void {}
"#;
    let once = oreslang_format::format_source_with_mtd(source).unwrap();
    assert!(once.contains("mtd foo() -> void {"), "{once}");
    assert!(once.contains("mtd bar() -> void ::"), "{once}");
    assert!(once.contains("static fnc helper() -> void {"), "{once}");
    assert!(once.contains("mtd mtd() -> void {"), "{once}");
    assert!(once.contains("suspend mtd work(): void {"), "{once}");
    assert!(once.contains("fnc standalone() -> void {"), "{once}");
    assert!(!once.contains("mtd mtd bar"), "{once}");
    assert_eq!(
        once,
        oreslang_format::format_source_with_mtd(&once).unwrap()
    );
}

#[test]
fn mtd_migration_handles_defaults_and_ignores_literals_and_comments() {
    let source = r#"define class Config as
// foo(int n = 2): void {}
pub calculate(int n = 2): int { return n; }
method(): void {}
mtd(): void {}
fnc old(): void {}
static fnc independent(): void {}
end
fnc top(): void {
  // mtd local(): void {}
  val text = "foo() -> void :: end";
}
"#;
    let migrated = oreslang_format::format_source_with_mtd(source).unwrap();
    assert!(migrated.contains("pub mtd calculate("), "{migrated}");
    assert!(migrated.contains("mtd method() -> void {"), "{migrated}");
    assert!(migrated.contains("mtd mtd() -> void {"), "{migrated}");
    assert!(migrated.contains("mtd old() -> void {"), "{migrated}");
    assert!(
        migrated.contains("static fnc independent() -> void {"),
        "{migrated}"
    );
    assert!(
        migrated.contains("// foo(int n = 2): void {}"),
        "{migrated}"
    );
    assert!(
        migrated.contains("val text = \"foo() -> void :: end\";"),
        "{migrated}"
    );
    assert!(!migrated.contains("mtd top()"), "{migrated}");
    assert_eq!(
        migrated,
        oreslang_format::format_source_with_mtd(&migrated).unwrap()
    );
}

#[test]
fn end_delimited_method_end_does_not_close_its_class() {
    let source = r#"define class Pair as
mtd first() -> int ::
if true then
return 1;
else
return 2;
fi
end
second(): int { return 3; }
end
"#;
    let formatted = format_source(source).unwrap();
    assert!(
        formatted.contains("  mtd first() -> int ::\n"),
        "{formatted}"
    );
    assert!(formatted.contains("    if true then\n"), "{formatted}");
    assert!(formatted.contains("      return 1;\n"), "{formatted}");
    assert!(
        formatted.contains("  end\n  second() -> int {"),
        "{formatted}"
    );
    assert_eq!(formatted, format_source(&formatted).unwrap());
}

#[test]
fn multiline_class_and_actor_methods_own_their_keyword_end() {
    let source = r#"define class C as
mtd sum(
int first,
int second
) -> int ::
return first + second;
end
next(): int { return 10; }
end
define actor Worker as
process(
int item
) -> int ::
return item;
end
later(): int { return 4; }
end
"#;
    let once = format_source(source).unwrap();
    assert!(once.contains(") -> int ::\n"), "{once}");
    assert!(once.contains("  end\n  next() -> int {"), "{once}");
    assert!(once.contains("  end\n  later(): int {"), "{once}");
    assert_eq!(once, format_source(&once).unwrap());
    let migrated = oreslang_format::format_source_with_mtd(source).unwrap();
    assert!(migrated.contains("mtd sum("), "{migrated}");
    assert!(migrated.contains("mtd process("), "{migrated}");
    assert!(migrated.contains("mtd later(): int {"), "{migrated}");
    assert_eq!(
        migrated,
        oreslang_format::format_source_with_mtd(&migrated).unwrap()
    );
}

#[test]
fn mtd_migration_distinguishes_generic_fields_from_generic_methods() {
    let source = r#"define class Storage as
List<int> items = []
Map<int, int> counts = {}
generic<T>(T value): T { return value; }
nested<T, List<int>>(T value): T { return value; }
end
"#;
    let migrated = oreslang_format::format_source_with_mtd(source).unwrap();
    assert!(!migrated.contains("mtd List"), "{migrated}");
    assert!(!migrated.contains("mtd Map"), "{migrated}");
    assert!(migrated.contains("mtd generic<T>("), "{migrated}");
    assert!(migrated.contains("mtd nested<T, List<int>>("), "{migrated}");
    assert_eq!(
        migrated,
        oreslang_format::format_source_with_mtd(&migrated).unwrap()
    );
}

#[test]
fn symbol_instance_method_migration_preserves_computed_method_name() {
    let source = r#"define class Iter as
[Symbol.iterator](): int { return 1; }
pub [Symbol.dispose]() -> void ::
end
mtd [Symbol.custom](): int { return 2; }
end
"#;
    let normal = format_source(source).unwrap();
    assert!(!normal.contains("mtd [Symbol.iterator]"), "{normal}");
    let once = oreslang_format::format_source_with_mtd(source).unwrap();
    assert!(once.contains("mtd [Symbol.iterator]() -> int {"), "{once}");
    assert!(
        once.contains("pub mtd [Symbol.dispose]() -> void ::"),
        "{once}"
    );
    assert!(once.contains("mtd [Symbol.custom]() -> int {"), "{once}");
    assert!(!once.contains("mtd mtd [Symbol.custom]"), "{once}");
    assert_eq!(
        once,
        oreslang_format::format_source_with_mtd(&once).unwrap()
    );
}

#[test]
fn failure_signatures_and_recovery_preserve_semantics_and_are_idempotent() {
    let source = "fnc fail(): void throws raises panics {\nraise 42;\n}\nfnc handle(): void {\ndefer || -> { recover(); };\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("throws raises panics"));
    assert!(formatted.contains("raise 42;"));
    assert!(formatted.contains("recover()"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn unchecked_call_markers_preserve_call_scope() {
    let source = "fnc use(): void {\nval result = unchecked service.read(argument());\nval future = await unchecked fetch();\nval handled = trap unchecked read();\n}\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("unchecked service.read(argument())"));
    assert!(formatted.contains("await unchecked fetch()"));
    assert!(formatted.contains("trap unchecked read()"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn explicit_legacy_method_prefix_canonicalizes_to_mtd_without_stealing_method_names() {
    let source = r#"define class C as
  pub method explicit(): int { return 2; }
  mtd already() -> int { return 3; }
  method(): int { return 4; }
  fnc legacy(): int { return 5; }
  fnc method(): int { return 8; }
  method method(): int { return 9; }
  static fnc helper(): int { return 6; }
end
pub fnc method(): int { return 7; }
"#;
    let formatted = oreslang_format::format_source(source).unwrap();
    assert!(formatted.contains("pub mtd explicit() -> int {"), "{formatted}");
    assert!(formatted.contains("mtd already() -> int {"), "{formatted}");
    assert!(formatted.contains("  method() -> int {"), "{formatted}");
    assert!(formatted.contains("  legacy() -> int {"), "{formatted}");
    assert!(formatted.contains("  method() -> int { return 8; }"), "{formatted}");
    assert!(formatted.contains("  mtd method() -> int { return 9; }"), "{formatted}");
    assert!(formatted.contains("static fnc helper() -> int {"), "{formatted}");
    assert!(formatted.contains("pub fnc method() -> int {"), "{formatted}");
    assert!(!formatted.contains("  method explicit()"), "{formatted}");
    assert_eq!(formatted, oreslang_format::format_source(&formatted).unwrap());
}
