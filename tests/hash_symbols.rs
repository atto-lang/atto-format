use oreslang_format::format_source;

#[test]
fn single_quoted_hash_symbols_are_canonicalized_without_colon_reinterpretation() {
    let src = "const Symbol status: #'ready foo';\nconst Symbol other: #ready;\n";
    let got = format_source(src).unwrap();
    assert!(got.contains("status: #\"ready foo\""), "{got}");
    assert!(got.contains("other: #ready"), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn quotes_and_escapes_preserve_the_decoded_symbol_name() {
    let src = r##"const Symbol a: #'one " two';
const Symbol b: #'can\'t';
const Symbol c: #'keep\u0061';
const Symbol d: #"already canonical";
"##;
    let got = format_source(src).unwrap();
    assert!(got.contains(r##"a: #"one \" two""##), "{got}");
    assert!(got.contains(r##"b: #"can't""##), "{got}");
    assert!(got.contains(r##"c: #"keep\u0061""##), "{got}");
    assert!(got.contains(r##"d: #"already canonical""##), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn hash_looking_text_inside_comments_or_strings_is_unchanged() {
    let src = r##"const String text: "#'not a symbol'";
// #'line comment'
/* #'block comment' */
const Symbol real: #'yes';
"##;
    let got = format_source(src).unwrap();
    assert!(got.contains(r##""#'not a symbol'""##), "{got}");
    assert!(got.contains("// #'line comment'"), "{got}");
    assert!(got.contains("/* #'block comment' */"), "{got}");
    assert!(got.contains(r##"real: #"yes""##), "{got}");
    assert_eq!(format_source(&got).unwrap(), got);
}

#[test]
fn malformed_hash_symbol_escapes_are_not_silently_repaired() {
    let src = r##"const Symbol invalid: #'bad\q';"##;
    let got = format_source(src).unwrap();
    assert!(got.contains(r##"#'bad\q'"##), "{got}");
}

#[test]
fn preserves_utf8_even_when_ordinary_string_contains_unrecognized_escape() {
    let src = r##"const String unchanged: "\é # '";
const Symbol actual: #'café';
"##;
    let got = format_source(src).unwrap();
    assert!(got.contains(r##""\é # '""##), "{got}");
    assert!(got.contains(r##"actual: #"café""##), "{got}");
}
