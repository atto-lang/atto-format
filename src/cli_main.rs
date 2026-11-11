use oreslang_format::{FormatError, format_source, format_source_with_mtd};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const HELP: &str = r#"oresfmt - the canonical Oreslang formatter

USAGE:
    oresfmt [--dry-run | --write | --check | --stdout | --analyze] [safety flags] <path>...
    oresfmt [--check] -

MODES:
    --dry-run   Preview changes without modifying files (default)
    --write     Rewrite files in place after Git safety checks
    --check     Do not write files; exit 1 if any input is not canonical
    --stdout    Print one formatted file to stdout instead of writing it
    --analyze   Run compiler static diagnostics without executing or modifying source
                Uses ORESLANG_BIN (default: oreslang-core) with --analyze
                Limits each file to ORESFMT_ANALYZE_TIMEOUT_SECS (default: 300)

MIGRATION:
    --insert-mtd  Insert optional 'mtd' before bare class/actor methods

WRITE SAFETY:
    --ok-to-mod-dirty-files
                Allow --write to modify tracked files with staged/unstaged changes
    --ok-to-mod-untracked-files
                Allow --write to modify untracked or ignored files
    --ok-to-mod-outside-git
                Allow --write outside a Git worktree

OTHER:
    -h, --help  Print help
    -V, --version
                Print version
    --          Stop option parsing; remaining arguments are filesystem paths

PATHS:
    Files are formatted directly. Directories are traversed recursively and
    tracked and untracked *.atto and *.ores files are selected.
    '-' reads stdin and writes stdout.

There are intentionally no general style flags or configuration files.
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    DryRun,
    Write,
    Check,
    Stdout,
    Analyze,
}

#[derive(Debug, Clone, Copy, Default)]
struct WritePolicy {
    allow_dirty_tracked: bool,
    allow_untracked: bool,
    allow_outside_git: bool,
}

#[derive(Debug)]
enum GitFileState {
    CleanTracked,
    DirtyTracked(String),
    UntrackedOrIgnored,
    OutsideGit,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("oresfmt: {err}");
            // Syntax/formatting rejection is an ordinary input failure.
            // Keep usage, Git-policy, filesystem and process failures distinct.
            ExitCode::from(if err.is::<FormatError>() { 1 } else { 2 })
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut mode = Mode::DryRun;
    let mut insert_mtd = false;
    let mut explicit_mode: Option<&'static str> = None;
    let mut policy = WritePolicy::default();
    let mut paths = Vec::new();
    let mut parse_options = true;

    for arg in env::args_os().skip(1) {
        if parse_options && arg == OsStr::new("--") {
            parse_options = false;
            continue;
        }

        if parse_options {
            if let Some(arg) = arg.to_str() {
                match arg {
                    "--dry-run" => {
                        select_mode(&mut mode, &mut explicit_mode, Mode::DryRun, "--dry-run")?
                    }
                    "--write" => {
                        select_mode(&mut mode, &mut explicit_mode, Mode::Write, "--write")?
                    }
                    "--check" => {
                        select_mode(&mut mode, &mut explicit_mode, Mode::Check, "--check")?
                    }
                    "--analyze" => {
                        select_mode(&mut mode, &mut explicit_mode, Mode::Analyze, "--analyze")?
                    }
                    "--stdout" => {
                        select_mode(&mut mode, &mut explicit_mode, Mode::Stdout, "--stdout")?
                    }
                    "--insert-mtd" => insert_mtd = true,
                    "--ok-to-mod-dirty-files" => policy.allow_dirty_tracked = true,
                    "--ok-to-mod-untracked-files" => policy.allow_untracked = true,
                    "--ok-to-mod-outside-git" => policy.allow_outside_git = true,
                    "-h" | "--help" => {
                        io::stdout().write_all(HELP.as_bytes())?;
                        return Ok(ExitCode::SUCCESS);
                    }
                    "-V" | "--version" => {
                        writeln!(io::stdout(), "oresfmt {}", env!("CARGO_PKG_VERSION"))?;
                        return Ok(ExitCode::SUCCESS);
                    }
                    _ if arg.starts_with('-') && arg != "-" => {
                        return Err(format!("unknown option: {arg}").into());
                    }
                    _ => {
                        paths.push(PathBuf::from(arg));
                        continue;
                    }
                }
                continue;
            }
        }

        // Non-UTF-8 arguments are valid filesystem paths. They cannot match
        // textual options, so preserve their raw OsString bytes.
        paths.push(PathBuf::from(arg));
    }

    if paths.is_empty() {
        return Err(
            "no input paths; pass one or more .ores files/directories, or '-' for stdin".into(),
        );
    }

    if mode != Mode::Write
        && (policy.allow_dirty_tracked || policy.allow_untracked || policy.allow_outside_git)
    {
        return Err("write-safety override flags require --write".into());
    }

    if mode == Mode::Analyze && insert_mtd {
        return Err("--insert-mtd cannot be combined with --analyze".into());
    }

    let has_stdin = paths
        .iter()
        .any(|path| path.as_os_str() == std::ffi::OsStr::new("-"));
    if has_stdin && paths.len() != 1 {
        return Err("'-' cannot be combined with filesystem paths".into());
    }

    if has_stdin {
        if mode == Mode::Write || mode == Mode::Analyze {
            return Err("--write/--analyze cannot be used with stdin".into());
        }

        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let formatted = if insert_mtd {
            format_source_with_mtd(&input)?
        } else {
            format_source(&input)?
        };

        if mode == Mode::Check {
            return Ok(if formatted == input {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            });
        }

        io::stdout().write_all(formatted.as_bytes())?;
        return Ok(ExitCode::SUCCESS);
    }

    if mode == Mode::Stdout {
        if paths.len() != 1 || !paths[0].is_file() {
            return Err("--stdout requires exactly one filesystem file".into());
        }
        let input = fs::read_to_string(&paths[0])?;
        let formatted = if insert_mtd {
            format_source_with_mtd(&input)?
        } else {
            format_source(&input)?
        };
        io::stdout().write_all(formatted.as_bytes())?;
        return Ok(ExitCode::SUCCESS);
    }

    // Exit 2 means "inconclusive" in analyze mode, so a run that analyzed nothing
    // must not be able to report it. Argument errors above remain usage errors.
    let files = match collect_files(&paths) {
        Ok(files) => files,
        Err(error) if mode == Mode::Analyze => return Ok(analysis_setup_failure(error)),
        Err(error) => return Err(error),
    };

    match mode {
        Mode::Analyze => Ok(run_analysis(files).unwrap_or_else(analysis_setup_failure)),
        Mode::DryRun => {
            for path in files {
                let input = fs::read_to_string(&path)?;
                let formatted = if insert_mtd {
                    format_source_with_mtd(&input)?
                } else {
                    format_source(&input)?
                };
                if formatted != input {
                    println!("would format {}", path.display());
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Mode::Check => {
            let mut dirty = false;
            for path in files {
                let input = fs::read_to_string(&path)?;
                let result = if insert_mtd {
                    format_source_with_mtd(&input)
                } else {
                    format_source(&input)
                };
                // A compiler-tip corpus can contain more than one unsupported
                // source file. Report *every* formatter rejection, not just
                // the first one, while preserving syntax failure exit code 1.
                // I/O failures still stop immediately with exit code 2.
                match result {
                    Ok(formatted) if formatted != input => {
                        eprintln!("needs formatting: {}", path.display());
                        dirty = true;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("oresfmt: rejected {}: {error}", path.display());
                        dirty = true;
                    }
                }
            }
            Ok(if dirty {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Mode::Write => write_files(files, policy, insert_mtd),
        Mode::Stdout => unreachable!("handled above"),
    }
}

fn select_mode(
    mode: &mut Mode,
    explicit_mode: &mut Option<&'static str>,
    next: Mode,
    flag: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(previous) = *explicit_mode {
        if previous != flag {
            return Err(format!("{previous} cannot be combined with {flag}").into());
        }
    }

    *mode = next;
    *explicit_mode = Some(flag);
    Ok(())
}

fn write_files(
    files: Vec<PathBuf>,
    policy: WritePolicy,
    insert_mtd: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut changes = Vec::new();

    for path in files {
        let input = fs::read_to_string(&path)?;
        let formatted = if insert_mtd {
            format_source_with_mtd(&input)?
        } else {
            format_source(&input)?
        };
        if formatted != input {
            changes.push((path, input, formatted));
        }
    }

    // Validate the entire write set first. A refusal on the last file must not
    // leave the workspace half-formatted.
    for (path, _, _) in &changes {
        enforce_write_policy(path, policy)?;
    }

    // Re-read the whole write set after Git preflight. This catches editors,
    // generators, hooks, or path substitutions that changed a target while
    // formatting was being prepared, before any write begins.
    for (path, original, _) in &changes {
        revalidate_write_target(path, original)?;
    }

    for (path, original, formatted) in changes {
        // Narrow the remaining TOCTOU window by validating each target again
        // immediately before its write. In particular, refuse if a path or one
        // of its parent directories was replaced by a symlink after preflight.
        revalidate_write_target(&path, &original)?;
        fs::write(&path, formatted)?;
        println!("formatted {}", path.display());
    }

    Ok(ExitCode::SUCCESS)
}

fn revalidate_write_target(
    path: &Path,
    original: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    reject_symlinked_ancestors(path)?;

    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("cannot re-inspect write target {}: {err}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "refusing to overwrite write target replaced by symlink: {}",
            path.display()
        )
        .into());
    }
    if !metadata.is_file() {
        return Err(format!(
            "refusing to overwrite write target that is no longer a regular file: {}",
            path.display()
        )
        .into());
    }

    let current = fs::read_to_string(path)?;
    if current.as_bytes() != original.as_bytes() {
        return Err(format!(
            "refusing to overwrite file changed during formatter preflight: {}",
            path.display()
        )
        .into());
    }

    Ok(())
}

fn reject_symlinked_ancestors(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    let mut current = if parent.is_absolute() {
        PathBuf::new()
    } else {
        env::current_dir()?
    };

    for component in parent.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                current.pop();
            }
            Component::Normal(part) => {
                current.push(part);
                let metadata = fs::symlink_metadata(&current).map_err(|err| {
                    format!(
                        "cannot inspect path component {} for {}: {err}",
                        current.display(),
                        path.display()
                    )
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(format!(
                        "refusing path through symlinked directory: {} (input {})",
                        current.display(),
                        path.display()
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}

fn enforce_write_policy(
    path: &Path,
    policy: WritePolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    match git_file_state(path)? {
        GitFileState::CleanTracked => Ok(()),
        GitFileState::DirtyTracked(status) if policy.allow_dirty_tracked => {
            eprintln!(
                "warning: modifying dirty tracked file: {} ({status})",
                path.display()
            );
            Ok(())
        }
        GitFileState::DirtyTracked(status) => Err(format!(
            "refusing to modify dirty tracked file: {} ({status}); commit/stash/revert it, or pass --ok-to-mod-dirty-files",
            path.display()
        )
        .into()),
        GitFileState::UntrackedOrIgnored if policy.allow_untracked => {
            eprintln!(
                "warning: modifying untracked/ignored file: {}",
                path.display()
            );
            Ok(())
        }
        GitFileState::UntrackedOrIgnored => Err(format!(
            "refusing to modify untracked or ignored file: {}; add+commit it, or pass --ok-to-mod-untracked-files",
            path.display()
        )
        .into()),
        GitFileState::OutsideGit if policy.allow_outside_git => {
            eprintln!(
                "warning: modifying file outside a Git worktree: {}",
                path.display()
            );
            Ok(())
        }
        GitFileState::OutsideGit => Err(format!(
            "refusing to modify file outside a Git worktree: {}; pass --ok-to-mod-outside-git to override",
            path.display()
        )
        .into()),
    }
}

fn git_file_state(path: &Path) -> Result<GitFileState, Box<dyn std::error::Error>> {
    let absolute = fs::canonicalize(path)?;
    let parent = absolute
        .parent()
        .ok_or_else(|| format!("cannot determine parent directory for {}", path.display()))?;

    let root_output = Command::new("git")
        .arg("-C")
        .arg(parent)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !root_output.status.success() {
        return Ok(GitFileState::OutsideGit);
    }

    let root_text = String::from_utf8(root_output.stdout)?;
    let root = fs::canonicalize(root_text.trim())?;
    let relative = absolute.strip_prefix(&root).map_err(|_| {
        format!(
            "Git reported worktree root {} but file {} is outside it",
            root.display(),
            absolute.display()
        )
    })?;

    let tracked = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(relative)
        .output()?;

    if !tracked.status.success() {
        return Ok(GitFileState::UntrackedOrIgnored);
    }

    let status = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain=v1", "--untracked-files=no", "--"])
        .arg(relative)
        .output()?;

    if !status.status.success() {
        return Err(format!("git status failed for {}", path.display()).into());
    }

    let status_text = String::from_utf8(status.stdout)?;
    let status_text = status_text.trim();
    if status_text.is_empty() {
        Ok(GitFileState::CleanTracked)
    } else {
        Ok(GitFileState::DirtyTracked(status_text.to_string()))
    }
}

/// Set on every compiler process so a wrapper, copy or hard link that leads back
/// to the formatter stops at one level instead of spawning without bound.
const ANALYZE_GUARD_ENV: &str = "ORESFMT_ANALYZE_ACTIVE";
const ANALYZE_TIMEOUT_ENV: &str = "ORESFMT_ANALYZE_TIMEOUT_SECS";
const DEFAULT_ANALYZE_TIMEOUT_SECS: u64 = 300;
const MAX_ANALYZE_TIMEOUT_SECS: u64 = 86_400;

fn analysis_setup_failure(error: Box<dyn std::error::Error>) -> ExitCode {
    eprintln!("oresfmt: {error}");
    ExitCode::from(1)
}

fn run_analysis(files: Vec<PathBuf>) -> Result<ExitCode, Box<dyn std::error::Error>> {
    if env::var_os(ANALYZE_GUARD_ENV).is_some() {
        return Err("compiler re-entered the formatter; recursive analysis refused".into());
    }
    let timeout = analysis_timeout()?;
    let requested = env::var_os("ORESLANG_BIN").unwrap_or_else(|| "oreslang-core".into());
    let compiler = resolve_compiler(Path::new(&requested))?;
    if fs::canonicalize(&compiler)? == fs::canonicalize(env::current_exe()?)? {
        return Err("ORESLANG_BIN resolves to the formatter; recursive analysis refused".into());
    }

    // Preserve the compiler-owned analysis outcomes across multiple files:
    // 0 = all verified in the backend's explicitly supported subset,
    // 1 = any source/compilation/process failure,
    // 2 = at least one inconclusive result, with no failures.
    // Never coerce "inconclusive" to either success or a syntax failure.
    let mut outcome: u8 = 0;
    for path in files {
        let mut command = Command::new(&compiler);
        command
            .arg("--analyze")
            .arg(compiler_path_argument(&path))
            .env(ANALYZE_GUARD_ENV, "1")
            .stdin(Stdio::null());
        let mut child = match spawn_retrying_busy(&mut command) {
            Ok(child) => child,
            Err(error) => {
                eprintln!(
                    "oresfmt: cannot run compiler {} for {}: {error}",
                    compiler.display(),
                    path.display()
                );
                outcome = 1;
                continue;
            }
        };
        match wait_with_timeout(&mut child, timeout)? {
            Some(status) if status.success() => {}
            Some(status) if status.code() == Some(2) => {
                eprintln!("oresfmt: analysis inconclusive: {}", path.display());
                if outcome == 0 {
                    outcome = 2;
                }
            }
            Some(_) => outcome = 1,
            None => {
                eprintln!(
                    "oresfmt: analysis timed out after {}s: {}",
                    timeout.as_secs(),
                    path.display()
                );
                outcome = 1;
            }
        }
    }
    Ok(ExitCode::from(outcome))
}

/// Retries briefly when the kernel reports the executable as open for writing.
/// A just-written wrapper script can hit this while any concurrently forked
/// process still holds the writer's descriptor.
fn spawn_retrying_busy(command: &mut Command) -> std::io::Result<Child> {
    // Make every Unix compiler and its ordinary descendants a separate signal
    // group, so a timeout cannot leave the compiler's direct children behind.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match command.spawn() {
            Err(error)
                if error.kind() == std::io::ErrorKind::ExecutableFileBusy
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            result => return result,
        }
    }
}

fn analysis_timeout() -> Result<Duration, Box<dyn std::error::Error>> {
    let Some(value) = env::var_os(ANALYZE_TIMEOUT_ENV) else {
        return Ok(Duration::from_secs(DEFAULT_ANALYZE_TIMEOUT_SECS));
    };
    match value.to_str().and_then(|text| text.parse::<u64>().ok()) {
        Some(seconds @ 1..=MAX_ANALYZE_TIMEOUT_SECS) => Ok(Duration::from_secs(seconds)),
        _ => Err(format!(
            "{ANALYZE_TIMEOUT_ENV} must be a whole number of seconds from 1 to {MAX_ANALYZE_TIMEOUT_SECS}"
        )
        .into()),
    }
}

/// Resolves the compiler once, so the program checked is the program executed.
/// A bare name only searches absolute PATH entries: an empty or `.` entry must
/// not let a checked-out repository supply the compiler.
fn resolve_compiler(requested: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if requested.as_os_str().is_empty() {
        return Err("ORESLANG_BIN is empty".into());
    }
    if requested.is_absolute() || requested.components().count() > 1 {
        return if is_executable_file(requested) {
            Ok(requested.to_path_buf())
        } else {
            Err(format!("compiler is not an executable file: {}", requested.display()).into())
        };
    }
    env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(requested))
        .find(|candidate| is_executable_file(candidate))
        .ok_or_else(|| {
            format!(
                "compiler not found on PATH: {}; set ORESLANG_BIN",
                requested.display()
            )
            .into()
        })
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// Keeps a source named like `-x.ores` from being parsed as a compiler option.
fn compiler_path_argument(path: &Path) -> PathBuf {
    if path.as_os_str().as_encoded_bytes().first() == Some(&b'-') {
        Path::new(".").join(path)
    } else {
        path.to_path_buf()
    }
}

/// Returns `None` after terminating an analyzer that exceeds its timeout.
/// Unix children are launched in a distinct process group, and the whole group
/// is signaled on timeout. This is not a sandbox: a hostile subprocess can
/// intentionally escape the group (for example, by creating a new session).
/// On non-Unix targets, only the direct process can currently be terminated.
fn wait_with_timeout(
    child: &mut Child,
    timeout: Duration,
) -> std::io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            // Kill background children still in our isolated Unix process
            // group before reaping the leader. Child::kill alone misses them.
            #[cfg(unix)]
            {
                unsafe extern "C" {
                    fn kill(pid: std::ffi::c_int, sig: std::ffi::c_int) -> std::ffi::c_int;
                }
                if let Ok(pid) = i32::try_from(child.id()) {
                    if pid > 1 {
                        // SAFETY: kill is the POSIX C ABI taking (pid_t, int).
                        // The negative PID addresses only our child's PGID,
                        // created by process_group(0) before the initial spawn.
                        let _ = unsafe { kill(-pid, 9) }; // SIGKILL
                    }
                }
            }
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn collect_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for path in inputs {
        collect_path(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err("no .atto or .ores files found".into());
    }
    Ok(files)
}

fn collect_path(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("cannot inspect {}: {err}", path.display()))?;

    if metadata.file_type().is_symlink() {
        return Err(format!(
            "refusing explicit symlink input: {}; pass the real path instead",
            path.display()
        )
        .into());
    }
    reject_symlinked_ancestors(path)?;
    if metadata.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("unsupported input path: {}", path.display()).into());
    }

    let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let child = entry.path();
        let name = entry.file_name();
        let file_type = entry.file_type()?;

        // Never follow symlinks discovered during a recursive walk. Besides
        // avoiding directory cycles, this prevents a repository-local path
        // from causing writes to a target outside the requested tree.
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() && matches!(name.to_str(), Some(".git" | "target" | ".cache")) {
            continue;
        }
        if file_type.is_dir() {
            collect_path(&child, out)?;
        } else if file_type.is_file() && child.extension().is_some_and(|ext| ext == "atto" || ext == "ores") {
            out.push(child);
        }
    }

    Ok(())
}

#[cfg(test)]
mod write_safety_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "oresfmt-write-safety-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        // macOS temp_dir traverses /var -> /private/var. Resolve only our
        // fixture root so the write guard reaches the mutation under test.
        fs::canonicalize(root).unwrap()
    }

    #[test]
    fn revalidation_rejects_changed_contents() {
        let root = scratch("changed");
        let path = root.join("sample.ores");
        fs::write(&path, "before").unwrap();
        fs::write(&path, "after").unwrap();

        let err = revalidate_write_target(&path, "before").unwrap_err();
        assert!(err.to_string().contains("changed during formatter preflight"));

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn revalidation_rejects_leaf_replaced_by_symlink() {
        use std::os::unix::fs::symlink;

        let root = scratch("leaf-symlink");
        let path = root.join("sample.ores");
        let target = root.join("target.ores");
        fs::write(&path, "before").unwrap();
        fs::write(&target, "external").unwrap();
        fs::remove_file(&path).unwrap();
        symlink(&target, &path).unwrap();

        let err = revalidate_write_target(&path, "before").unwrap_err();
        assert!(err.to_string().contains("replaced by symlink"));
        assert_eq!(fs::read_to_string(target).unwrap(), "external");

        let _ = fs::remove_dir_all(root);
    }
}
