use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const INPUT: &str = "pub fnc main() -> int {\n  val x = 10;\n  x = 20;\n  return   x;\n}\n";
const EXPECTED: &str = "pub fnc main() -> int {\n  val x = 10;\n  x = 20;\n  return x;\n}\n";

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "oresfmt-return-spacing-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn return_spacing_cli_stdout_check_and_explicit_write_are_consistent() {
    let directory = TempDirectory::new();
    let source = directory.0.join("main.atto");
    fs::write(&source, INPUT).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&directory.0)
        .args(["--stdout", "main.atto"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), EXPECTED);
    assert_eq!(fs::read_to_string(&source).unwrap(), INPUT);

    let check = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&directory.0)
        .args(["--check", "main.atto"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1), "{check:?}");
    assert_eq!(fs::read_to_string(&source).unwrap(), INPUT);

    // A temporary file outside Git must still require explicit write consent.
    let denied = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&directory.0)
        .args(["--write", "main.atto"])
        .output()
        .unwrap();
    assert_eq!(denied.status.code(), Some(2), "{denied:?}");
    assert_eq!(fs::read_to_string(&source).unwrap(), INPUT);

    let written = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&directory.0)
        .args(["--write", "--ok-to-mod-outside-git", "main.atto"])
        .output()
        .unwrap();
    assert!(written.status.success(), "{written:?}");
    assert_eq!(fs::read_to_string(&source).unwrap(), EXPECTED);

    let clean = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&directory.0)
        .args(["--check", "main.atto"])
        .output()
        .unwrap();
    assert!(clean.status.success(), "{clean:?}");
}

#[test]
fn stdin_modes_apply_same_return_spacing_without_touching_files() {
    for (input, check_code, expected_stdout) in [(INPUT, 1, EXPECTED), (EXPECTED, 0, EXPECTED)] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected_stdout);

        let mut child = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .args(["--check", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let checked = child.wait_with_output().unwrap();
        assert_eq!(checked.status.code(), Some(check_code), "{checked:?}");
        assert!(checked.stdout.is_empty());
    }
}

#[test]
fn write_override_does_not_modify_syntactically_invalid_return_source() {
    let directory = TempDirectory::new();
    let original = "pub fnc broken() -> int {\nreturn    1;\n";
    let source = directory.0.join("broken.atto");
    fs::write(&source, original).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&directory.0)
        .args(["--write", "--ok-to-mod-outside-git", "broken.atto"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(fs::read_to_string(&source).unwrap(), original);
}
