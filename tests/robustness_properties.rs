//! Robustness properties checked over a mixed AttoLang sample:
//! - every prefix (truncated at each char boundary) formats or is rejected,
//!   never panics (a multibyte boundary once panicked in continuation logic);
//! - indentation is the formatter's job: removing all leading whitespace
//!   yields the same canonical output;
//! - canonical output is idempotent.

use oreslang_format::format_source;

const SAMPLES: &[&str] = &[
    "define module m as\n  define class Box as\n    pub val int a: 1;\n    int b = 2;\n    pub read(): int {\n      return self.a;\n    }\n  end\nend\n",
    "define actor W as {\n  let total: int = 0;\n  pub fnc on(int n): int {\n    if (n eq 0) { return 0; } fi\n    return total;\n  }\n}\n",
    "fnc f(): bool {\n  val m = [\n    1,\n    2\n  ];\n  return a(m) == \"中\" &&\n    b() eq中 ||\n    c();\n}\n",
    "fnc g(): int {\n  for let i=0;i<3;i++ do if x(i) then return 0;fi\n  y();\n  done\n  if p {\n    q();\n  } else {\n    r();\n  } fi\n  return 1;\n}\n",
    "define contract C as\n  fnc add(int a) -> int;\nend\npub actor Echo\n  receive(int m): void {\n    return;\n  }\nend\nfnc gen* values(): Iterator<int> {\n  yield 1;\n  return;\n}\n",
];

#[test]
fn every_prefix_formats_or_rejects_without_panicking() {
    for sample in SAMPLES {
        for (at, _) in sample.char_indices() {
            let _ = format_source(&sample[..at]);
        }
    }
}

#[test]
fn leading_whitespace_does_not_change_canonical_output() {
    for sample in SAMPLES {
        let canonical = format_source(sample).unwrap_or_else(|err| panic!("{err}\n{sample}"));
        let flat: String = sample
            .lines()
            .map(|line| line.trim_start())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(format_source(&flat).unwrap(), canonical, "{sample}");
        assert_eq!(format_source(&canonical).unwrap(), canonical, "{sample}");
    }
}

#[test]
fn bare_cr_is_rejected_but_crlf_is_normalized() {
    // Comment boundaries at a lone CR differ between the spec and compilers.
    let hidden =
        "// comment\rimport fnc helper from \"+/src/../lib\";\nfnc main(): int { return 1; }\n";
    let err = format_source(hidden).unwrap_err();
    assert!(err.message().contains("bare CR"), "{}", err.message());
    let crlf = "fnc main(): int {\r\n  return 1;\r\n}\r\n";
    assert_eq!(
        format_source(crlf).unwrap(),
        "fnc main() -> int {\n  return 1;\n}\n"
    );
}
