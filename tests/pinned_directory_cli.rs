#![cfg(target_os = "linux")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const INPUT: &str = "fnc answer() => int {\nreturn 42;\n}\n";
const EXPECTED: &str = "fnc answer() -> int {\n  return 42;\n}\n";

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "oresfmt-dirfd-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let init = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(init.status.success());
        Self { root }
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn pinned_write_refuses_parent_traversal_without_touching_source() {
    let work = Scratch::new("traversal");
    fs::create_dir(work.root.join("sub")).unwrap();
    let source = work.root.join("sample.ores");
    fs::write(&source, INPUT).unwrap();

    let result = work.run(&[
        "--write",
        "--ok-to-mod-untracked-files",
        "sub/../sample.ores",
    ]);
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&source).unwrap(), INPUT);
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("unsafe parent directory component"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn pinned_write_completes_and_leaves_no_staging_entries() {
    let work = Scratch::new("clean-staging");
    let directory = work.root.join("src");
    fs::create_dir(&directory).unwrap();
    let source = directory.join("sample.ores");
    fs::write(&source, INPUT).unwrap();

    let result = work.run(&["--write", "--ok-to-mod-untracked-files", "src/sample.ores"]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(fs::read_to_string(&source).unwrap(), EXPECTED);
    let entries: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], "sample.ores");
}
