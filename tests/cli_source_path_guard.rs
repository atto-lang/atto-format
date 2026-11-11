//! Guard CLI modes against accidental rewriting of compiler-adjacent files.
//! Source discovery is .atto-first with intentional .ores compatibility.
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "oresfmt-source-guard-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test workspace");
        Self(path)
    }

    fn file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, content).expect("write fixture");
        path
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .expect("run formatter")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const DIRTY: &str = "fnc answer() => int {\nreturn 42;\n}\n";

#[test]
fn atto_and_legacy_ores_are_accepted_by_all_read_only_modes() {
    let work = Scratch::new("source-extensions");
    work.file("source.atto", DIRTY);
    work.file("legacy.ores", DIRTY);
    for name in ["source.atto", "legacy.ores"] {
        let stdout = work.run(&["--stdout", name]);
        assert!(stdout.status.success(), "{stdout:?}");
        assert_eq!(
            String::from_utf8(stdout.stdout).expect("UTF-8"),
            "fnc answer() -> int {\n  return 42;\n}\n"
        );
        let check = work.run(&["--check", name]);
        assert_eq!(check.status.code(), Some(1), "{check:?}");
    }
}

#[test]
fn unsupported_explicit_extensions_fail_closed_in_every_mode() {
    let work = Scratch::new("unsupported-extension");
    let target = work.file("source.rs", DIRTY);
    for mode in ["--stdout", "--check", "--dry-run", "--write"] {
        let output = work.run(&[mode, "source.rs"]);
        assert_eq!(output.status.code(), Some(2), "{mode}: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unsupported source extension"),
            "{mode}: {output:?}"
        );
        assert!(output.stdout.is_empty(), "{mode} wrote output");
        assert_eq!(fs::read_to_string(&target).unwrap(), DIRTY);
    }
}

#[test]
fn directory_discovery_skips_non_source_files() {
    let work = Scratch::new("source-discovery");
    work.file("source.atto", DIRTY);
    work.file("source.rs", DIRTY);
    let output = work.run(&["--dry-run", "."]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("source.atto"), "{stdout}");
    assert!(!stdout.contains("source.rs"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn stdout_refuses_symlink_file_and_symlink_parent() {
    use std::os::unix::fs::symlink;
    let work = Scratch::new("stdout-symlinks");
    let source = work.file("source.atto", DIRTY);
    symlink(&source, work.0.join("alias.atto")).expect("create file symlink");
    fs::create_dir(work.0.join("real")).expect("create directory");
    work.file("real/nested.atto", DIRTY);
    symlink(work.0.join("real"), work.0.join("linked")).expect("create dir symlink");

    for (path, diagnostic) in [
        ("alias.atto", "refusing explicit symlink input"),
        (
            "linked/nested.atto",
            "refusing path through symlinked directory",
        ),
    ] {
        let output = work.run(&["--stdout", path]);
        assert_eq!(output.status.code(), Some(2), "{path}: {output:?}");
        assert!(output.stdout.is_empty(), "{path} wrote partial source");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(diagnostic),
            "{path}: {output:?}"
        );
    }
}
