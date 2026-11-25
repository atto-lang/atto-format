//! CLI behavior when some inputs are rejected: rejection is exit 1 and names
//! the file, `--write` stays all-or-nothing, and non-source files are ignored.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "oresfmt-reject-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        for args in [
            &["init", "-q"][..],
            &["config", "user.name", "oresfmt-tests"],
            &["config", "user.email", "oresfmt-tests@example.invalid"],
        ] {
            git(&root, args);
        }
        Self { root }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn commit(&self) {
        git(&self.root, &["add", "."]);
        git(&self.root, &["commit", "-qm", "baseline"]);
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

const NEEDS_FORMAT: &str = "fnc a() => int {\nreturn 1;\n}\n";
const BARE_CR: &str =
    "// hidden\rimport fnc helper from \"+/src/../lib\";\nfnc b(): int { return 2; }\n";
const SYNTAX_ERROR: &str = "fnc broken() -> int {\n  if x then\n    return 1;\n}\n";

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn bare_cr_is_rejected_by_check_and_write_and_names_the_file() {
    let repo = Repo::new("bare-cr");
    let path = repo.write("src/b.atto", BARE_CR);
    repo.commit();

    let check = repo.run(&["--check", "src/b.atto"]);
    assert_eq!(check.status.code(), Some(1));
    assert!(
        stderr(&check).contains("src/b.atto: line 1: bare CR"),
        "{}",
        stderr(&check)
    );

    let write = repo.run(&["--write", "src/b.atto"]);
    assert_eq!(write.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        BARE_CR,
        "rejected file was modified"
    );
}

#[test]
fn stdin_rejection_is_exit_one_without_a_path() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(BARE_CR.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "rejected input must not produce output"
    );
    assert!(
        stderr(&output).starts_with("oresfmt: line 1: bare CR"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn write_is_all_or_nothing_when_any_file_is_rejected() {
    for (label, rejected) in [("cr", BARE_CR), ("syntax", SYNTAX_ERROR)] {
        let repo = Repo::new(&format!("atomic-{label}"));
        let good = repo.write("a.atto", NEEDS_FORMAT);
        let nested = repo.write("sub/c.atto", NEEDS_FORMAT);
        let bad = repo.write("sub/z.atto", rejected);
        repo.commit();

        let output = repo.run(&["--write", "."]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{label}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("z.atto"),
            "{label}: {}",
            stderr(&output)
        );
        assert_eq!(fs::read_to_string(good).unwrap(), NEEDS_FORMAT, "{label}");
        assert_eq!(fs::read_to_string(nested).unwrap(), NEEDS_FORMAT, "{label}");
        assert_eq!(fs::read_to_string(bad).unwrap(), rejected, "{label}");
    }
}

#[test]
fn recursive_walk_formats_atto_and_ores_and_ignores_other_files() {
    let repo = Repo::new("discovery");
    let atto = repo.write("deep/x/main.atto", NEEDS_FORMAT);
    let ores = repo.write("legacy.ores", NEEDS_FORMAT);
    let txt = repo.write("notes.txt", NEEDS_FORMAT);
    let md = repo.write("deep/README.md", NEEDS_FORMAT);
    repo.commit();

    let check = repo.run(&["--check", "."]);
    assert_eq!(check.status.code(), Some(1));
    let err = stderr(&check);
    assert!(
        err.contains("main.atto") && err.contains("legacy.ores"),
        "{err}"
    );
    assert!(
        !err.contains("notes.txt") && !err.contains("README.md"),
        "{err}"
    );

    let write = repo.run(&["--write", "."]);
    assert!(write.status.success(), "{}", stderr(&write));
    let canonical = "fnc a() -> int {\n  return 1;\n}\n";
    assert_eq!(fs::read_to_string(atto).unwrap(), canonical);
    assert_eq!(fs::read_to_string(ores).unwrap(), canonical);
    assert_eq!(fs::read_to_string(txt).unwrap(), NEEDS_FORMAT);
    assert_eq!(fs::read_to_string(md).unwrap(), NEEDS_FORMAT);
    assert!(repo.run(&["--check", "."]).status.success());
}

#[test]
fn already_canonical_files_are_not_rewritten() {
    let repo = Repo::new("untouched");
    let canonical = "fnc a() -> int {\n  return 1;\n}\n";
    let path = repo.write("a.atto", canonical);
    repo.commit();
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));

    let write = repo.run(&["--write", "a.atto"]);
    assert!(write.status.success());
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
}

#[test]
fn stdout_mode_names_the_rejected_file() {
    let repo = Repo::new("stdout");
    repo.write("bad.atto", SYNTAX_ERROR);
    let output = repo.run(&["--stdout", "bad.atto"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains("bad.atto: line"),
        "{}",
        stderr(&output)
    );
}
