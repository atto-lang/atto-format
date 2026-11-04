#[cfg(unix)]
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("oresfmt-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.name", "oresfmt-tests"]);
        git(
            &root,
            &["config", "user.email", "oresfmt-tests@example.invalid"],
        );
        Self { root }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    fn commit_all(&self, message: &str) {
        git(&self.root, &["add", "."]);
        git(&self.root, &["commit", "-qm", message]);
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn run_os(&self, args: &[&OsStr]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_oresfmt"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

const UNFORMATTED: &str = "fnc answer() => int {\nreturn 42;\n}\n";
const FORMATTED: &str = "fnc answer() -> int {\n  return 42;\n}\n";

#[test]
fn dry_run_is_default_and_never_writes() {
    let repo = TempRepo::new("dry-run");
    let path = repo.write("sample.ores", UNFORMATTED);

    let output = repo.run(&["sample.ores"]);

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(path).unwrap(), UNFORMATTED);
    assert!(String::from_utf8_lossy(&output.stdout).contains("would format sample.ores"));
}

#[test]
fn check_is_nonzero_but_does_not_write() {
    let repo = TempRepo::new("check");
    let path = repo.write("sample.ores", UNFORMATTED);

    let output = repo.run(&["--check", "sample.ores"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(path).unwrap(), UNFORMATTED);
    assert!(String::from_utf8_lossy(&output.stderr).contains("needs formatting"));
}

#[test]
fn write_allows_clean_tracked_file() {
    let repo = TempRepo::new("clean-tracked");
    let path = repo.write("sample.ores", UNFORMATTED);
    repo.commit_all("baseline");

    let output = repo.run(&["--write", "sample.ores"]);

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(path).unwrap(), FORMATTED);
}

#[test]
fn unused_import_check_dry_run_and_write_are_consistent() {
    let repo = TempRepo::new("unused-import-cli");
    let source = "// oresfmt: pure-import\nimport class Old from \"./old\";\nimport class Used from \"./used\";\npub fnc main(): void {\n  Used();\n  return;\n}\n";
    let target = repo.write("example.ores", source);
    repo.commit_all("baseline");

    let dry = repo.run(&["example.ores"]);
    assert!(dry.status.success());
    assert_eq!(fs::read_to_string(&target).unwrap(), source);
    assert!(String::from_utf8_lossy(&dry.stdout).contains("would format example.ores"));

    let check = repo.run(&["--check", "example.ores"]);
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&target).unwrap(), source);

    let write = repo.run(&["--write", "example.ores"]);
    assert!(write.status.success());
    let formatted = fs::read_to_string(&target).unwrap();
    assert!(formatted.contains("\n// import class Old from"));
    assert!(formatted.contains("\nimport class Used from"));
    assert!(formatted.contains("  Used();"));
    assert!(repo.run(&["--check", "example.ores"]).status.success());
}

#[test]
fn write_refuses_dirty_tracked_file_by_default() {
    let repo = TempRepo::new("dirty-tracked");
    let path = repo.write("sample.ores", UNFORMATTED);
    repo.commit_all("baseline");
    repo.write("sample.ores", "fnc answer() => int {\nreturn 43;\n}\n");
    let before = fs::read_to_string(&path).unwrap();

    let output = repo.run(&["--write", "sample.ores"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(path).unwrap(), before);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to modify dirty tracked file")
    );
}

#[test]
fn dirty_tracked_override_is_explicit() {
    let repo = TempRepo::new("dirty-override");
    let path = repo.write("sample.ores", UNFORMATTED);
    repo.commit_all("baseline");
    repo.write("sample.ores", "fnc answer() => int {\nreturn 43;\n}\n");

    let output = repo.run(&["--write", "--ok-to-mod-dirty-files", "sample.ores"]);

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        "fnc answer() -> int {\n  return 43;\n}\n"
    );
}

#[test]
fn write_refuses_untracked_file_unless_explicitly_allowed() {
    let repo = TempRepo::new("untracked");
    let path = repo.write("sample.ores", UNFORMATTED);

    let refused = repo.run(&["--write", "sample.ores"]);
    assert_eq!(refused.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&path).unwrap(), UNFORMATTED);
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("refusing to modify untracked or ignored file")
    );

    let allowed = repo.run(&["--write", "--ok-to-mod-untracked-files", "sample.ores"]);
    assert!(allowed.status.success());
    assert_eq!(fs::read_to_string(path).unwrap(), FORMATTED);
}

#[test]
fn write_preflights_every_change_before_touching_any_file() {
    let repo = TempRepo::new("atomic-preflight");
    let tracked = repo.write("a.ores", UNFORMATTED);
    repo.commit_all("tracked baseline");
    let untracked = repo.write("b.ores", UNFORMATTED);

    let output = repo.run(&["--write", "."]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(tracked).unwrap(), UNFORMATTED);
    assert_eq!(fs::read_to_string(untracked).unwrap(), UNFORMATTED);
}

#[test]
fn write_outside_git_fails_closed_without_override() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "oresfmt-outside-git-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("sample.ores");
    fs::write(&path, UNFORMATTED).unwrap();

    let refused = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&root)
        .args(["--write", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&path).unwrap(), UNFORMATTED);

    let allowed = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&root)
        .args(["--write", "--ok-to-mod-outside-git", "sample.ores"])
        .output()
        .unwrap();
    assert!(allowed.status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), FORMATTED);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn write_safety_overrides_require_write_mode() {
    let repo = TempRepo::new("override-mode");
    repo.write("sample.ores", UNFORMATTED);

    let output = repo.run(&["--check", "--ok-to-mod-untracked-files", "sample.ores"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("write-safety override flags require --write")
    );
}

#[cfg(unix)]
#[test]
fn explicit_symlink_inputs_are_refused() {
    use std::os::unix::fs::symlink;

    let repo = TempRepo::new("symlink");
    let target = repo.write("target.ores", UNFORMATTED);
    let link = repo.root.join("link.ores");
    symlink(&target, &link).unwrap();

    let output = repo.run(&["link.ores"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(target).unwrap(), UNFORMATTED);
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing explicit symlink input"));
}

#[cfg(unix)]
#[test]
fn recursive_walk_skips_symlinked_ores_files() {
    use std::os::unix::fs::symlink;

    let repo = TempRepo::new("walk-symlink");
    let real = repo.write("real.ores", FORMATTED);
    let external_root = std::env::temp_dir().join(format!(
        "oresfmt-external-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&external_root).unwrap();
    let external = external_root.join("external.ores");
    fs::write(&external, UNFORMATTED).unwrap();
    symlink(&external, repo.root.join("linked.ores")).unwrap();

    let output = repo.run(&["."]);

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(real).unwrap(), FORMATTED);
    assert_eq!(fs::read_to_string(&external).unwrap(), UNFORMATTED);

    let _ = fs::remove_dir_all(external_root);
}

#[cfg(unix)]
#[test]
fn explicit_paths_through_symlinked_directories_are_refused() {
    use std::os::unix::fs::symlink;

    let repo = TempRepo::new("symlink-parent");
    let real_dir = repo.root.join("real");
    fs::create_dir_all(&real_dir).unwrap();
    let target = real_dir.join("sample.ores");
    fs::write(&target, UNFORMATTED).unwrap();
    let alias = repo.root.join("alias");
    symlink(&real_dir, &alias).unwrap();

    let output = repo.run(&["alias/sample.ores"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(target).unwrap(), UNFORMATTED);
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("refusing path through symlinked directory")
    );
}

#[test]
fn end_of_options_allows_dash_prefixed_paths() {
    let repo = TempRepo::new("dash-path");
    let path = repo.write("-sample.ores", UNFORMATTED);

    let refused = repo.run(&["--check", "-sample.ores"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("unknown option"));

    let accepted = repo.run(&["--check", "--", "-sample.ores"]);
    assert_eq!(accepted.status.code(), Some(1));
    assert_eq!(fs::read_to_string(path).unwrap(), UNFORMATTED);
    assert!(String::from_utf8_lossy(&accepted.stderr).contains("needs formatting"));
}

#[test]
fn analyze_missing_compiler_fails_without_changing_source() {
    let repo = TempRepo::new("analysis-missing-compiler");
    let path = repo.write("sample.ores", UNFORMATTED);
    let output = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&repo.root)
        .env("ORESLANG_BIN", repo.root.join("missing-compiler"))
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(path).unwrap(), UNFORMATTED);
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_are_preserved_as_os_strings() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let repo = TempRepo::new("non-utf8-path");
    let name = OsString::from_vec(b"sample-\xff.ores".to_vec());
    let path = repo.root.join(&name);
    let created = fs::write(&path, UNFORMATTED);
    let args = [OsStr::new("--check"), OsStr::new("--"), name.as_os_str()];

    let output = repo.run_os(&args);

    if let Err(error) = created {
        // APFS rejects invalid UTF-8 names before the CLI can inspect them.
        // Still exercise OsString argument handling and require a clean error.
        assert_eq!(error.raw_os_error(), Some(92)); // EILSEQ on Darwin
        assert_eq!(output.status.code(), Some(2));
        assert!(!path.exists());
    } else {
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(fs::read_to_string(path).unwrap(), UNFORMATTED);
    }
}

#[test]
fn insert_mtd_is_an_opt_in_migration_not_a_default() {
    let repo = TempRepo::new("insert-mtd");
    let path = repo.write("methods.ores", "define class X as\nfoo(): void {}\nend\n");
    repo.commit_all("baseline");

    let default = repo.run(&["--stdout", "methods.ores"]);
    assert!(default.status.success());
    assert!(!String::from_utf8_lossy(&default.stdout).contains("mtd foo()"));

    let explicit = repo.run(&["--insert-mtd", "--stdout", "methods.ores"]);
    assert!(explicit.status.success());
    assert!(String::from_utf8_lossy(&explicit.stdout).contains("mtd foo()"));

    let written = repo.run(&["--write", "--insert-mtd", "methods.ores"]);
    assert!(written.status.success());
    assert!(fs::read_to_string(&path).unwrap().contains("mtd foo()"));
    let checked = repo.run(&["--check", "--insert-mtd", "methods.ores"]);
    assert!(checked.status.success());
}

#[cfg(unix)]
#[test]
fn analyze_delegates_to_compiler_and_preserves_failure_status() {
    use std::os::unix::fs::PermissionsExt;
    let repo = TempRepo::new("analysis-compiler");
    let path = repo.write("sample.ores", UNFORMATTED);
    let compiler = repo.write("compiler", "#!/bin/sh\n[ \"$1\" = --analyze ] || exit 9\n[ \"$2\" = sample.ores ] || exit 9\necho 'failure-warning: missing raises marker' >&2\nexit 1\n");
    fs::set_permissions(&compiler, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&repo.root)
        .env("ORESLANG_BIN", compiler)
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failure-warning"));
    assert_eq!(fs::read_to_string(path).unwrap(), UNFORMATTED);
}

#[test]
fn analyze_refuses_recursive_compiler_configuration() {
    let repo = TempRepo::new("analysis-recursion");
    repo.write("sample.ores", UNFORMATTED);
    let output = Command::new(env!("CARGO_BIN_EXE_oresfmt"))
        .current_dir(&repo.root)
        .env("ORESLANG_BIN", env!("CARGO_BIN_EXE_oresfmt"))
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("recursive analysis refused"));
}

#[cfg(unix)]
fn analyze_command(repo: &TempRepo) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_oresfmt"));
    command
        .current_dir(&repo.root)
        .env_remove("ORESFMT_ANALYZE_ACTIVE")
        .env_remove("ORESFMT_ANALYZE_TIMEOUT_SECS");
    command
}

#[cfg(unix)]
fn write_script(repo: &TempRepo, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = repo.write(name, &format!("#!/bin/sh\n{body}\n"));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[cfg(unix)]
#[test]
fn analyze_dash_prefixed_source_cannot_become_a_compiler_option() {
    let repo = TempRepo::new("analysis-dash-path");
    repo.write("-x.ores", UNFORMATTED);
    let compiler = write_script(&repo, "compiler", r#"[ "$2" = ./-x.ores ] || exit 9"#);
    let output = analyze_command(&repo)
        .env("ORESLANG_BIN", compiler)
        .args(["--analyze", "--", "-x.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn analyze_gives_the_compiler_a_closed_stdin() {
    use std::io::Write;
    use std::process::Stdio;
    let repo = TempRepo::new("analysis-stdin");
    repo.write("sample.ores", UNFORMATTED);
    let compiler = write_script(&repo, "compiler", r#"[ -z "$(/bin/cat)" ] || exit 9"#);
    let mut child = analyze_command(&repo)
        .env("ORESLANG_BIN", compiler)
        .args(["--analyze", "sample.ores"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"secret\n").unwrap();
    drop(stdin);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[cfg(unix)]
#[test]
fn analyze_wrapper_leading_back_to_the_formatter_stops_at_one_level() {
    let repo = TempRepo::new("analysis-wrapper-recursion");
    repo.write("sample.ores", UNFORMATTED);
    // The depth cap keeps a regression from spawning without bound.
    let wrapper = write_script(
        &repo,
        "wrapper",
        r#"echo x >> depth
[ "$(/usr/bin/wc -l < depth)" -ge 4 ] && exit 0
exec "$FORMATTER" "$@""#,
    );
    let output = analyze_command(&repo)
        .env("ORESLANG_BIN", wrapper)
        .env("FORMATTER", env!("CARGO_BIN_EXE_oresfmt"))
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(repo.root.join("depth")).unwrap(), "x\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("recursive analysis refused"));
}

#[cfg(unix)]
#[test]
fn analyze_path_search_skips_non_executable_shadows() {
    let repo = TempRepo::new("analysis-path-shadow");
    repo.write("sample.ores", UNFORMATTED);
    repo.write("first/oreslang-core", "not executable");
    write_script(&repo, "second/oreslang-core", "echo ran > ran");
    let path = format!("{0}/first:{0}/second", repo.root.display());
    let output = analyze_command(&repo)
        .env("PATH", path)
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(repo.root.join("ran").exists());
}

#[cfg(unix)]
#[test]
fn analyze_refuses_the_formatter_found_on_path_through_a_symlink() {
    use std::os::unix::fs::symlink;
    let repo = TempRepo::new("analysis-path-symlink");
    repo.write("sample.ores", UNFORMATTED);
    fs::create_dir_all(repo.root.join("tools")).unwrap();
    symlink(
        env!("CARGO_BIN_EXE_oresfmt"),
        repo.root.join("tools/oreslang-core"),
    )
    .unwrap();
    let output = analyze_command(&repo)
        .env("PATH", repo.root.join("tools"))
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("recursive analysis refused"));
}

#[cfg(unix)]
#[test]
fn analyze_non_executable_shadow_cannot_hide_the_formatter_from_the_self_check() {
    use std::os::unix::fs::symlink;
    let repo = TempRepo::new("analysis-shadowed-self");
    repo.write("sample.ores", UNFORMATTED);
    repo.write("first/oreslang-core", "not executable");
    fs::create_dir_all(repo.root.join("second")).unwrap();
    symlink(
        env!("CARGO_BIN_EXE_oresfmt"),
        repo.root.join("second/oreslang-core"),
    )
    .unwrap();
    let path = format!("{0}/first:{0}/second", repo.root.display());
    let output = analyze_command(&repo)
        .env("PATH", path)
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("resolves to the formatter"));
}

#[cfg(unix)]
#[test]
fn analyze_ignores_relative_and_empty_path_entries() {
    let repo = TempRepo::new("analysis-relative-path");
    repo.write("sample.ores", UNFORMATTED);
    write_script(&repo, "oreslang-core", "echo ran > ran");
    for path in [".", ""] {
        let output = analyze_command(&repo)
            .env("PATH", path)
            .args(["--analyze", "sample.ores"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("not found on PATH"));
        assert!(!repo.root.join("ran").exists());
    }
}

#[cfg(unix)]
#[test]
fn analyze_times_out_a_hung_compiler_and_still_checks_later_files() {
    let repo = TempRepo::new("analysis-timeout");
    repo.write("a.ores", UNFORMATTED);
    repo.write("b.ores", UNFORMATTED);
    let compiler = write_script(
        &repo,
        "compiler",
        r#"[ "$2" = a.ores ] && exec /bin/sleep 30
echo "$2" >> seen"#,
    );
    let started = std::time::Instant::now();
    let output = analyze_command(&repo)
        .env("ORESLANG_BIN", compiler)
        .env("ORESFMT_ANALYZE_TIMEOUT_SECS", "3")
        .args(["--analyze", "a.ores", "b.ores"])
        .output()
        .unwrap();
    assert!(started.elapsed().as_secs() < 20);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("analysis timed out after 3s: a.ores")
    );
    assert_eq!(
        fs::read_to_string(repo.root.join("seen")).unwrap(),
        "b.ores\n"
    );
}

#[cfg(unix)]
#[test]
fn analyze_rejects_invalid_timeouts_before_running_anything() {
    let repo = TempRepo::new("analysis-invalid-timeout");
    repo.write("sample.ores", UNFORMATTED);
    let compiler = write_script(&repo, "compiler", "echo ran > ran");
    for value in ["0", "-1", "soon", "86401", ""] {
        let output = analyze_command(&repo)
            .env("ORESLANG_BIN", &compiler)
            .env("ORESFMT_ANALYZE_TIMEOUT_SECS", value)
            .args(["--analyze", "sample.ores"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "timeout {value:?}");
        assert!(!repo.root.join("ran").exists());
    }
}

#[cfg(unix)]
#[test]
fn analyze_refuses_a_non_executable_compiler_path() {
    let repo = TempRepo::new("analysis-non-executable");
    repo.write("sample.ores", UNFORMATTED);
    let compiler = repo.write("compiler", "not executable");
    let output = analyze_command(&repo)
        .env("ORESLANG_BIN", compiler)
        .args(["--analyze", "sample.ores"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not an executable file"));
}

#[test]
fn analyze_rejects_insert_mtd() {
    let repo = TempRepo::new("analysis-insert-mtd");
    repo.write("sample.ores", UNFORMATTED);
    let output = repo.run(&["--analyze", "--insert-mtd", "sample.ores"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--insert-mtd cannot be combined"));
}
