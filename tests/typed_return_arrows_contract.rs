use oreslang_format::{format_source, is_formatted};

fn format_twice(source: &str) -> String {
    let once = format_source(source).expect("typed return syntax must format");
    let twice = format_source(&once).expect("formatted source must reformat");
    assert_eq!(once, twice, "formatter must be idempotent");
    assert!(is_formatted(&once).unwrap(), "formatted output must be stable");
    once
}

#[test]
fn explicit_lambda_return_and_defer_void_are_not_dispatch_results() {
    let source = r#"fnc f(): int {
const fnc get = || -> int { return 42; };
defer || -> void { stdio.println("defer done"); };
return get();
}
"#;
    let output = format_twice(source);
    assert!(output.contains("|| -> int {"), "{output}");
    assert!(output.contains("|| -> void {"), "{output}");
    assert!(output.contains("defer || -> void"), "{output}");
    assert!(!output.contains("=> void"), "{output}");
}

#[test]
fn typed_match_value_result_inside_typed_lambda_keeps_two_return_scopes() {
    let source = r#"fnc f(bool ready): int {
const fnc compute = || -> int {
return match<int> ready over
on true -> 41;
on false -> 1;
end;
};
return compute() + 1;
}
"#;
    let output = format_twice(source);
    for exact in ["|| -> int {", "return match<int> ready over",
                  "on true -> 41;", "on false -> 1;", "return compute() + 1;"] {
        assert!(output.contains(exact), "lost {exact}:\n{output}");
    }
}

#[test]
fn explicit_switch_and_select_arm_return_types_survive_formatting() {
    let sources = [
        r#"fnc classify(int code): string {
return switch<string> code over
case 200 -> string { return "ok"; }
default -> string { return "error"; }
end;
}
"#,
        r#"fnc read(Channel<int> values): string {
return select<string> over
when readch values: int item -> string { return "selected"; }
end;
}
"#,
        r#"fnc classify(int code): string {
return switch<string> code over
case 200 -> { return "ok"; }
default -> { return "error"; }
end;
}
"#,
    ];
    for source in sources {
        let output = format_twice(source);
        assert!(
            output.contains("<string>") && output.contains("->"),
            "lost typed dispatch syntax:\n{output}"
        );
        if source.contains("-> string {") {
            assert!(output.contains("-> string {"), "lost typed arm:\n{output}");
        }
        if source.contains("int item -> string {") {
            assert!(
                output.contains("int item -> string {"),
                "channel payload and result types were conflated:\n{output}"
            );
        }
    }
}

#[test]
fn comments_and_literals_with_fake_return_arrows_are_not_executable_arms() {
    let source = r#"fnc f(): void {
const string text = "return switch<int> x over case 1 -> void {";
 // || -> int { } and when readch c: int n -> string {
defer || -> void { stdio.println(text); };
return;
}
"#;
    let output = format_twice(source);
    assert!(
        output.contains("\"return switch<int> x over case 1 -> void {\""),
        "{output}"
    );
    assert!(
        output.contains("// || -> int { } and when readch c: int n -> string {"),
        "{output}"
    );
}
