//! Executable CLI coverage for mixed .atto/.ores compiler-head corpora.
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Corpus {
    root: PathBuf,
}

impl Corpus {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "oresfmt-corpus-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn add(&self, name: &str, source: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        path
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Corpus {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const NONCANONICAL: &str = "fnc answer() => int {\nreturn 42;\n}\n";
const MALFORMED: &str = "fnc broken() -> int {\n  return 1;\n";

#[test]
fn recursive_check_reports_every_unsupported_source_and_every_noncanonical_file() {
    let corpus = Corpus::new("mixed-check");
    let atto = corpus.add("src/one.atto", NONCANONICAL);
    let legacy = corpus.add("src/deeper/two.ores", NONCANONICAL);
    let invalid_atto = corpus.add("src/bad/three.atto", MALFORMED);
    let invalid_ores = corpus.add("src/bad/four.ores", MALFORMED);
    corpus.add("src/ignored.txt", NONCANONICAL);

    for binary in [
        env!("CARGO_BIN_EXE_oresfmt"),
        env!("CARGO_BIN_EXE_oreslang-format"),
    ] {
        let output = Command::new(binary)
            .current_dir(&corpus.root)
            .args(["--check", "src"])
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1), "{binary}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        for name in ["one.atto", "two.ores"] {
            assert!(stderr.contains("needs formatting:"), "{stderr}");
            assert!(stderr.contains(name), "missing {name}: {stderr}");
        }
        for name in ["three.atto", "four.ores"] {
            assert!(
                stderr
                    .lines()
                    .any(|line| line.contains("rejected") && line.contains(name)),
                "missing source rejection for {name}: {stderr}"
            );
        }
        assert!(!stderr.contains("ignored.txt"), "{stderr}");
    }

    assert_eq!(fs::read_to_string(atto).unwrap(), NONCANONICAL);
    assert_eq!(fs::read_to_string(legacy).unwrap(), NONCANONICAL);
    assert_eq!(fs::read_to_string(invalid_atto).unwrap(), MALFORMED);
    assert_eq!(fs::read_to_string(invalid_ores).unwrap(), MALFORMED);
}

#[test]
fn check_clean_atto_and_ores_tree_returns_zero() {
    let corpus = Corpus::new("mixed-clean");
    corpus.add("src/one.atto", "fnc answer() -> int {\n  return 42;\n}\n");
    corpus.add(
        "src/sub/two.ores",
        "fnc answer() -> int {\n  return 42;\n}\n",
    );
    let output = corpus.run(&["--check", "src"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn write_aborts_before_modifying_any_file_when_later_atto_is_invalid() {
    let corpus = Corpus::new("mixed-write");
    let valid = corpus.add("src/a.ores", NONCANONICAL);
    let invalid = corpus.add("src/z.atto", MALFORMED);
    let output = corpus.run(&["--write", "src"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(valid).unwrap(), NONCANONICAL);
    assert_eq!(fs::read_to_string(invalid).unwrap(), MALFORMED);
    assert!(output.stdout.is_empty());
}

#[test]
fn check_keeps_io_failures_distinct_from_source_rejections() {
    let corpus = Corpus::new("mixed-io");
    corpus.add("src/a.atto", NONCANONICAL);
    fs::write(corpus.root.join("src/z.ores"), [0xff, 0xfe]).unwrap();
    let output = corpus.run(&["--check", "src"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("needs formatting:"));
}
