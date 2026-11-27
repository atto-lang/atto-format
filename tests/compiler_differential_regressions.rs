//! Regressions found by running the formatter over the oreslang-source.* and
//! oreslang-platform.* corpora and comparing what the Java (`--check`) and
//! LLVM (`oreslang-llvmc`) compilers accept before and after formatting.
//! See scripts/check_formatter_acceptance_delta.py. Each case cites the
//! compiler behavior it protects; none of these tests is compiler evidence.

use oreslang_format::format_source;

fn fmt(source: &str) -> String {
    let once = format_source(source).unwrap_or_else(|err| panic!("{err}\n{source}"));
    assert_eq!(format_source(&once).unwrap(), once, "not idempotent");
    once
}

// Java: bare type-first fields are LET (parseTypeFirstClassField), LLVM: `val`
// sets final slot + mutable referent. Dropping `val` turned the rejected
// write-to-immutable fixture class-val-alias-write-reject into an accepted one.
#[test]
fn val_fields_are_never_elided() {
    let src = "define class Pair as\n  pub val int a: 13;\n  pub val String method;\nend\n";
    let out = fmt(src);
    assert!(out.contains("pub val int a: 13;"), "{out}");
    // Java parses an uninitialized bare `String method;` as a method header.
    assert!(out.contains("pub val String method;"), "{out}");
}

// Java parseField accepts only `=` after val/let/const.
#[test]
fn binding_keyword_fields_keep_equals_initializers() {
    let src = "define class A as\n  pub let int v = 10;\n  const int c = 1;\n  val int w = 2;\n  int bare = 3;\n  final mut int z = 4;\nend\n";
    let out = fmt(src);
    for keep in ["pub let int v = 10;", "const int c = 1;", "val int w = 2;"] {
        assert!(out.contains(keep), "{keep} missing in {out}");
    }
    assert!(out.contains("int bare: 3;"), "{out}");
    assert!(out.contains("final mut int z: 4;"), "{out}");
}

// Java interface parser: match(FAT_ARROW, COLON); LLVM: "contract signatures
// require contiguous '=>'".
#[test]
fn contract_signatures_use_fat_arrow() {
    let src = "define module contracts as\n  define contract MathApi as\n    fnc add(int a, int b) -> int;\n    fnc neg(int a): int;\n  end\nend\n";
    let out = fmt(src);
    assert!(out.contains("fnc add(int a, int b) => int;"), "{out}");
    assert!(out.contains("fnc neg(int a) => int;"), "{out}");
}

// Java rejects a top-level bare `gen*`; LLVM rejects `fnc gen*` and
// `generator fnc`. No rewrite between them is accepted by both.
#[test]
fn generator_headers_are_preserved() {
    for header in [
        "async generator fnc events() -> int {",
        "fnc gen* values(): Iterator<int> {",
        "pub gen* values(): Iterator<int> {",
    ] {
        let src = format!("{header}\nyield 1;\nreturn;\n}}\n");
        let out = fmt(&src);
        assert!(out.starts_with(header), "{out}");
        assert!(out.contains("\n  yield 1;\n"), "{out}");
    }
}

// LLVM actor callables require `:` before the return type; Java accepts both.
#[test]
fn actor_callables_use_colon_return_separator() {
    let src = "define actor Worker as {\n  let total: int = 0;\n  pub fnc on_message(int n) -> int {\n    return n;\n  }\n}\n";
    let out = fmt(src);
    assert!(out.contains("  pub on_message(int n): int {"), "{out}");
}

// LLVM treats a top-level `suspend fnc` as a cooperative loop (requiring `:`)
// and rejects `suspends fnc`; Java aliases the two spellings.
#[test]
fn top_level_suspend_spelling_and_colon_are_kept() {
    let out = fmt("suspend fnc spin():void {\n  return;\n}\n");
    assert!(out.starts_with("suspend fnc spin(): void {"), "{out}");
    let out = fmt("suspends fnc spin():void {\n  return;\n}\n");
    assert!(out.starts_with("suspends fnc spin() -> void {"), "{out}");
}

// Legacy LLVM fixtures use `} fi`; canonical brace forms are self-delimiting.
#[test]
fn brace_form_conditionals_close_with_same_line_fi() {
    let out = fmt("fnc f(): int {\nif (x eq 1) { return 1; } fi\nreturn 0;\n}\n");
    assert!(
        out.contains("  if (x eq 1) { return 1; } fi\n  return 0;"),
        "{out}"
    );

    let out = fmt(
        "fnc f(): int {\nif x eq 1 {\nreturn 1;\n} elif x eq 2 {\nreturn 2;\n} else {\nreturn 3;\n} fi\n}\n",
    );
    assert!(
        out.contains("  } else {\n    return 3;\n  } fi\n}"),
        "{out}"
    );

    // Canonical no-fi must round-trip; a stray `} fi` remains rejected.
    let canonical = fmt("fnc f(): int {\nif x {\nreturn 1;\n}\nreturn 0;\n}\n");
    assert!(canonical.contains("  }\n  return 0;"), "{canonical}");
    assert_eq!(format_source(&canonical).unwrap(), canonical);
    assert!(format_source("fnc f(): int {\nif x then\nreturn 1;\n} fi\n}\n").is_err());
}

// Java Lexer and LLVM both nest block comments.
#[test]
fn block_comments_nest() {
    let out =
        fmt("/* beginning /* nested */ end */\npub fnc main(): int {\n    return 4_0 + 2;\n}\n");
    assert!(
        out.starts_with("/* beginning /* nested */ end */\npub fnc main() -> int {\n  return"),
        "{out}"
    );
    let out = fmt("fnc f(): int {\n  return 1; /* a /* if */ fi */\n}\n");
    assert!(out.contains("return 1; /* a /* if */ fi */"), "{out}");
    assert!(format_source("pub fnc main(): int { return 42; } /* open /* nested */\n").is_err());
}

// `x == "id" && y` used to compact to `==&&` and be rejected as unary `&`.
#[test]
fn string_literals_do_not_create_false_unary_borrow_matches() {
    let out = fmt("fnc f(): bool {\n  return a() == \"id\"&&b();\n}\n");
    assert!(out.contains("== \"id\"&&b()"), "{out}");
    assert!(format_source("fnc f(): int {\n  return g(&x);\n}\n").is_err());
}

// A callable whose parameters were expanded one-per-line used to lose one of
// its two separating blank lines on the second pass.
#[test]
fn expanded_parameter_headers_keep_callable_separation_idempotently() {
    let src = "fnc a(): int {\n  return 1;\n}\nfnc web_leaf(String root, String dir, String url, String kind, bool check, bool dry): String {\n  return root;\n}\n";
    let out = fmt(src);
    assert!(
        out.contains("}\n\n\nfnc web_leaf(\n  String root,"),
        "{out}"
    );
}

#[test]
fn compact_return_type_gets_a_space_before_the_body() {
    let out = fmt("fnc main(old:int,delta:int):int{return old+delta;}\n");
    assert!(
        out.starts_with("fnc main(int old, int delta) -> int {\n"),
        "{out}"
    );
}

#[test]
fn multiline_expressions_keep_one_continuation_level() {
    let src = "fnc f(): int {\nval x = foo(\na,\nb\n);\nval m = [\n1,\n2\n];\nitems.each(|x| -> {\nprint(x);\n});\nreturn obj\n.method()\n.other() +\n\"tail\";\n}\n";
    let expected = "fnc f() -> int {\n  val x = foo(\n    a,\n    b\n  );\n  val m = [\n    1,\n    2\n  ];\n  items.each(|x| -> {\n    print(x);\n  });\n  return obj\n    .method()\n    .other() +\n    \"tail\";\n}\n";
    assert_eq!(fmt(src), expected);
}

#[test]
fn trailing_string_is_not_a_continuation_operator() {
    let src = "define class Foo as\n  bar: string = \"typed\"\n  update() -> void {\n  }\nend\n";
    let out = fmt(src);
    assert!(out.contains("\n  update() -> void {"), "{out}");
}

// Java accepts these; the formatter used to reject them (wide-corpus audit).
#[test]
fn compact_do_loop_headers_trait_compat_order_and_bare_actor_headers() {
    let out = fmt(
        "fnc f(): int {\nfor let i=0;i<3;i++ do if a(i) then return 0;fi\nb();\ndone\nreturn 1;\n}\n",
    );
    assert!(out.contains("  for let i=0;i<3;i++ do\n    if a(i) then\n      return 0;\n    fi\n    b();\n  done\n"), "{out}");
    let out = fmt("fnc f(): int {\nwhile x do y(); done\nreturn 1;\n}\n");
    assert!(out.contains("  while x do y(); done\n"), "{out}");
    let out = fmt("pub define trait Scored as\nfnc score() => int;\nend\n");
    assert!(out.contains("\n  fnc score() => int;\nend"), "{out}");
    let out = fmt("pub actor Echo\nreceive(int m): void {\nself.end();\nreturn;\n}\nend\n");
    assert!(
        out.contains("  receive(int m): void {\n    self.end();"),
        "{out}"
    );
}

// End-delimited actor callables keep `->`: the formatter only recognizes
// `-> T ::` headers, so `: T ::` failed on the second pass.
#[test]
fn end_delimited_actor_callables_keep_the_slim_arrow() {
    let out = fmt("define actor Counter as\n  start() -> void ::\n    go();\n  end\nend\n");
    assert!(
        out.contains("  start() -> void ::\n    go();\n  end"),
        "{out}"
    );
}
