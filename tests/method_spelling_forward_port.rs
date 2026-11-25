use oreslang_format::{format_source, is_formatted};

#[test]
fn direct_instance_fnc_uses_current_method_contract_without_touching_static_or_module() {
    let input = "define class Counter as\n  pub fnc inc(int value) => int {\n    return value;\n  }\n  static fnc count() => int {\n    return 1;\n  }\nend\n\nfnc global(int x) => int {\n  return x;\n}\n";
    let result = format_source(input).unwrap();
    assert!(result.contains("pub inc("), "{result}");
    assert!(!result.contains("pub fnc inc("), "{result}");
    assert!(result.contains("static fnc count("), "{result}");
    assert!(result.contains("fnc global("), "{result}");
    assert!(is_formatted(&result).unwrap());
    assert_eq!(format_source(&result).unwrap(), result);
}

#[test]
fn method_spelling_inside_quoted_string_is_not_rewritten() {
    let input = "define class Counter as\n  pub fnc method_name() => String {\n    return \"fnc fake() -> void\";\n  }\nend\n";
    let result = format_source(input).unwrap();
    assert!(result.contains("\"fnc fake() -> void\""), "{result}");
    assert!(result.contains("pub method_name("), "{result}");
}

#[test]
fn generic_class_factory_and_members_omit_fnc_but_free_and_static_retain_it() {
    let input = r#"define pub class Container[T]
    implements Newable[Container[T]] as
  pub fnc new(): Container[T] {
    return new Container[T]();
  }
  val int count = 21;
  val bool ready = true;
  val string label = "ok";
  pub fnc echo(T value): T {
    return value;
  }
  pub fnc get_count(): int {
    return self.count;
  }
  pub fnc is_ready(): bool {
    return self.ready;
  }
  pub fnc get_label(): string {
    return self.label;
  }
  static fnc helper(): int {
    return 42;
  }
end
pub fnc main(): int {
  return 42;
}
"#;
    let output = format_source(input).unwrap();
    for name in ["new", "echo", "get_count", "is_ready", "get_label"] {
        assert!(output.contains(&format!("pub {name}(")), "{output}");
        assert!(!output.contains(&format!("pub fnc {name}(")), "{output}");
    }
    assert!(output.contains("static fnc helper("), "{output}");
    assert!(output.contains("pub fnc main("), "{output}");
    assert_eq!(format_source(&output).unwrap(), output);
}
