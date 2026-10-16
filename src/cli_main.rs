use oreslang_format::{FormatError, format_source, is_formatted};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};

const HELP: &str = r#"oresfmt - the canonical Oreslang formatter

USAGE:
    oresfmt [--dry-run | --write | --check | --stdout] [safety flags] <path>...
    oresfmt [--check] -

MODES:
    --dry-run   Preview changes without modifying files (default)
    --write     Rewrite files in place after Git safety checks
    --check     Do not write files; exit 1 if any input is not canonical
    --stdout    Print one formatted file to stdout instead of writing it

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
    only *.ores files are selected. '-' reads stdin and writes stdout.

There are intentionally no style flags or configuration files.
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    DryRun,
    Write,
    Check,
    Stdout,
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
            // Source syntax/formatting rejections are distinct from tool errors.
            ExitCode::from(if err.is::<FormatError>() { 1 } else { 2 })
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut mode = Mode::DryRun;
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
                    "--stdout" => {
                        select_mode(&mut mode, &mut explicit_mode, Mode::Stdout, "--stdout")?
                    }
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

    let has_stdin = paths
        .iter()
        .any(|path| path.as_os_str() == std::ffi::OsStr::new("-"));
    if has_stdin && paths.len() != 1 {
        return Err("'-' cannot be combined with filesystem paths".into());
    }

    if has_stdin {
        if mode == Mode::Write {
            return Err("--write cannot be used with stdin".into());
        }

        let input = read_bounded_source(&mut io::stdin(), "stdin")?;
        let formatted = format_source(&input)?;

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
        if paths.len() != 1 {
            return Err("--stdout requires exactly one filesystem file".into());
        }
        let metadata = fs::symlink_metadata(&paths[0])?;
        if metadata.file_type().is_symlink() {
            return Err(format!("refusing explicit symlink input: {}", paths[0].display()).into());
        }
        reject_symlinked_ancestors(&paths[0])?;
        if !metadata.is_file() {
            return Err("--stdout requires exactly one filesystem file".into());
        }
        let input = read_regular_source(&paths[0])?;
        let formatted = format_source(&input)?;
        io::stdout().write_all(formatted.as_bytes())?;
        return Ok(ExitCode::SUCCESS);
    }

    let files = collect_files(&paths)?;

    match mode {
        Mode::DryRun => {
            for path in files {
                let input = read_regular_source(&path)?;
                if !is_formatted(&input)? {
                    println!("would format {}", path.display());
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Mode::Check => {
            let mut dirty = false;
            for path in files {
                let input = read_regular_source(&path)?;
                if !is_formatted(&input)? {
                    eprintln!("needs formatting: {}", path.display());
                    dirty = true;
                }
            }
            Ok(if dirty {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Mode::Write => write_files(files, policy),
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


// Limit reads even if a file grows while a live descriptor is being consumed.
// The same upper bound applies to stdin and read-only CLI modes.
const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

fn read_bounded_source(
    input: &mut impl Read,
    label: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    input.take(MAX_SOURCE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err(format!(
            "refusing source exceeding {MAX_SOURCE_BYTES} bytes: {label}"
        ).into());
    }
    Ok(String::from_utf8(bytes)?)
}

fn read_regular_source(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A FIFO or device substituted after path preflight must not block.
        // Refuse symlink leaves and only consume verified regular file handles.
        const O_NONBLOCK: i32 = 0o4000;
        const O_NOFOLLOW: i32 = 0o400000;
        const O_CLOEXEC: i32 = 0o2000000;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(format!(
                "refusing non-regular Oreslang source: {}",
                path.display()
            ).into());
        }
        read_bounded_source(&mut file, &path.display().to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut file = fs::File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(format!("refusing non-regular Oreslang source: {}", path.display()).into());
        }
        read_bounded_source(&mut file, &path.display().to_string())
    }
}

fn write_files(
    files: Vec<PathBuf>,
    policy: WritePolicy,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut changes = Vec::new();

    for path in files {
        let original_metadata = fs::symlink_metadata(&path)?;
        let input = read_regular_source(&path)?;
        #[cfg(unix)]
        check_source_identity(&path, &original_metadata)?;
        let formatted = format_source(&input)?;
        if formatted != input {
            changes.push((path, input, formatted, original_metadata));
        }
    }

    // Validate the entire write set first. A refusal on the last file must not
    // leave the workspace half-formatted.
    for (path, _, _, _) in &changes {
        enforce_write_policy(path, policy)?;
    }

    // Preflight metadata for every changed file before writing any file.
    // In particular, refuse ACL/xattr-bearing files to prevent lossy renames.
    for (path, _, _, metadata) in &changes {
        reject_unsupported_file_metadata(path)?;
        #[cfg(unix)]
        check_source_identity(path, metadata)?;
    }

    // Re-read the whole write set after Git preflight. This catches editors,
    // generators, hooks, or path substitutions that changed a target while
    // formatting was being prepared, before any write begins.
    for (path, original, _, metadata) in &changes {
        revalidate_write_target(path, original)?;
        #[cfg(unix)]
        check_source_identity(path, metadata)?;
    }

    for (path, original, formatted, metadata) in changes {
        // Narrow the remaining TOCTOU window by validating each target again
        // immediately before its write. In particular, refuse if a path or one
        // of its parent directories was replaced by a symlink after preflight.
        revalidate_write_target(&path, &original)?;
        #[cfg(unix)]
        check_source_identity(&path, &metadata)?;
        replace_atomically(&path, &original, &formatted, &metadata)?;
        println!("formatted {}", path.display());
    }

    Ok(ExitCode::SUCCESS)
}


#[cfg(unix)]
fn check_source_identity(
    path: &Path,
    original: &fs::Metadata,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::MetadataExt;

    let current = fs::symlink_metadata(path)?;
    if !current.is_file()
        || current.dev() != original.dev()
        || current.ino() != original.ino()
        || current.uid() != original.uid()
        || current.gid() != original.gid()
        || current.mode() != original.mode()
        || current.size() != original.size()
        || current.mtime() != original.mtime()
        || current.mtime_nsec() != original.mtime_nsec()
        || current.ctime() != original.ctime()
        || current.ctime_nsec() != original.ctime_nsec()
    {
        return Err(format!(
            "refusing source inode or metadata changed during preflight: {}",
            path.display()
        )
        .into());
    }
    Ok(())
}

#[cfg(unix)]
fn check_parent_identity(
    parent: &Path,
    original: &fs::Metadata,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::MetadataExt;
    let current = fs::symlink_metadata(parent)?;
    if !current.is_dir() || current.dev() != original.dev() || current.ino() != original.ino() {
        return Err(format!(
            "refusing directory replaced during atomic preflight: {}",
            parent.display()
        ).into());
    }
    Ok(())
}


#[cfg(target_os = "linux")]
mod pinned_directory {
    use super::*;
    use std::ffi::{CString, OsStr};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::raw::{c_char, c_int};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;

    const O_WRONLY: c_int = 1;
    const O_RDONLY: c_int = 0;
    const O_CREAT: c_int = 0o100;
    const O_EXCL: c_int = 0o200;
    const O_NONBLOCK: c_int = 0o4000;
    const O_DIRECTORY: c_int = 0o200000;
    const O_NOFOLLOW: c_int = 0o400000;
    const O_CLOEXEC: c_int = 0o2000000;

    unsafe extern "C" {
        fn openat(dirfd: c_int, pathname: *const c_char, flags: c_int, mode: u32) -> c_int;
        fn renameat(
            olddirfd: c_int,
            oldpath: *const c_char,
            newdirfd: c_int,
            newpath: *const c_char,
        ) -> c_int;
        fn unlinkat(dirfd: c_int, pathname: *const c_char, flags: c_int) -> c_int;
        fn flistxattr(fd: c_int, list: *mut c_char, size: usize) -> isize;
        fn flock(fd: c_int, operation: c_int) -> c_int;
    }

    fn checked_name(name: &OsStr) -> io::Result<CString> {
        CString::new(name.as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "embedded NUL in name"))
    }

    fn open_relative(dir: &fs::File, name: &OsStr, flags: c_int, mode: u32) -> io::Result<fs::File> {
        let name = checked_name(name)?;
        // SAFETY: name is NUL-terminated and dir is a live owned descriptor.
        let fd = unsafe { openat(dir.as_raw_fd(), name.as_ptr(), flags, mode) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful openat returned a new owned file descriptor.
        Ok(unsafe { fs::File::from_raw_fd(fd) })
    }

    pub(super) fn open_parent(parent: &Path) -> io::Result<fs::File> {
        let mut dir = fs::File::open(if parent.is_absolute() { "/" } else { "." })?;
        for component in parent.components() {
            match component {
                Component::RootDir | Component::CurDir => {}
                Component::Normal(name) => {
                    dir = open_relative(
                        &dir,
                        name,
                        O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
                        0,
                    )?;
                }
                // Refuse parent traversal rather than resolving it through
                // a pathname outside the pinned directory descent.
                Component::ParentDir | Component::Prefix(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unsafe parent directory component in write path",
                    ));
                }
            }
        }
        Ok(dir)
    }

    pub(super) fn try_lock_directory(dir: &fs::File) -> io::Result<()> {
        // Advisory only: cooperating formatter instances serialize their
        // same-directory writes, and contention fails without blocking.
        const LOCK_EX: c_int = 2;
        const LOCK_NB: c_int = 4;
        // SAFETY: dir is a live file descriptor and flock reads no pointers.
        if unsafe { flock(dir.as_raw_fd(), LOCK_EX | LOCK_NB) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn stage(dir: &fs::File, name: &OsStr) -> io::Result<fs::File> {
        open_relative(
            dir,
            name,
            O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
            0o600,
        )
    }

    pub(super) fn ensure_no_extended_attributes(
        file: &fs::File,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // SAFETY: a null buffer and zero capacity only query the attribute
        // list length for this live descriptor. Any metadata is unsupported.
        let count = unsafe { flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0) };
        if count < 0 {
            return Err(io::Error::last_os_error().into());
        }
        if count != 0 {
            return Err("refusing staged file with inherited extended attributes or ACLs".into());
        }
        Ok(())
    }

    pub(super) fn confirm_source(
        dir: &fs::File,
        name: &OsStr,
        expected: &fs::Metadata,
        original: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut source = open_relative(
            dir,
            name,
            O_RDONLY | O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC,
            0,
        )?;
        let found = source.metadata()?;
        if !found.is_file()
            || found.dev() != expected.dev()
            || found.ino() != expected.ino()
            || found.uid() != expected.uid()
            || found.gid() != expected.gid()
            || found.mode() != expected.mode()
            || found.size() != expected.size()
            || found.mtime() != expected.mtime()
            || found.mtime_nsec() != expected.mtime_nsec()
            || found.ctime() != expected.ctime()
            || found.ctime_nsec() != expected.ctime_nsec()
        {
            return Err("refusing source inode or metadata changed via pinned directory".into());
        }
        ensure_no_extended_attributes(&source)?;
        let current = read_bounded_source(&mut source, "pinned source")?;
        if current != original {
            return Err("refusing source bytes changed via pinned directory".into());
        }
        Ok(())
    }

    pub(super) fn replace(dir: &fs::File, temp: &OsStr, target: &OsStr) -> io::Result<()> {
        let temp = checked_name(temp)?;
        let target = checked_name(target)?;
        // SAFETY: descriptors are live, names are NUL-terminated, and both
        // paths are relative to the same pinned directory descriptor.
        let result = unsafe {
            renameat(
                dir.as_raw_fd(),
                temp.as_ptr(),
                dir.as_raw_fd(),
                target.as_ptr(),
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn cleanup(dir: &fs::File, temp: &OsStr) -> io::Result<()> {
        let temp = checked_name(temp)?;
        // SAFETY: descriptor is live and the temporary basename is valid.
        let result = unsafe { unlinkat(dir.as_raw_fd(), temp.as_ptr(), 0) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn reject_unsupported_file_metadata(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::fd::AsRawFd;
    use std::os::raw::{c_char, c_int};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    unsafe extern "C" {
        fn flistxattr(fd: c_int, list: *mut c_char, size: usize) -> isize;
    }

    // Linux O_NOFOLLOW: never inspect an unintended symlink destination.
    const O_NOFOLLOW: i32 = 0o400000;
    const O_NONBLOCK: i32 = 0o4000;
    const O_CLOEXEC: i32 = 0o2000000;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
        .open(path)?;
    let fd_metadata = file.metadata()?;
    let path_metadata = fs::symlink_metadata(path)?;
    if !path_metadata.is_file()
        || fd_metadata.dev() != path_metadata.dev()
        || fd_metadata.ino() != path_metadata.ino()
    {
        return Err(format!("refusing source path replaced while inspecting metadata: {}", path.display()).into());
    }

    // A null buffer with zero capacity asks only for the required byte count.
    // This inspects all Linux namespaces, including ACL and security xattrs.
    let bytes = unsafe { flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0) };
    if bytes < 0 {
        return Err(format!(
            "cannot verify source extended attributes for {}: {}",
            path.display(),
            io::Error::last_os_error()
        ).into());
    }
    if bytes > 0 {
        return Err(format!(
            "refusing atomic replacement of file with extended attributes or ACLs: {}",
            path.display()
        ).into());
    }
    Ok(())
}

// Platforms without a verified metadata-preserving replacement implementation
// must fail closed rather than silently strip ACLs or extended attributes.
#[cfg(not(target_os = "linux"))]
fn reject_unsupported_file_metadata(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Err(format!(
        "atomic --write is not yet validated for metadata preservation on this platform: {}",
        path.display()
    ).into())
}

// Avoid truncating the destination on short writes, disk exhaustion or other
// write errors. Replacing the directory entry also avoids mutating an
// unexpected hard-linked file outside the caller's requested path.
//
// This is a per-file atomic replacement, NOT a multi-file transaction.
// Symlinked parent paths are preflighted, but fully hostile concurrent
// directory replacement requires dirfd/openat-style confinement.
fn replace_atomically(
    path: &Path,
    original: &str,
    formatted: &str,
    initial_metadata: &fs::Metadata,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{SystemTime, UNIX_EPOCH};

    revalidate_write_target(path, original)?;
    reject_unsupported_file_metadata(path)?;
    #[cfg(unix)]
    check_source_identity(path, initial_metadata)?;
    let original_metadata = fs::symlink_metadata(path)?;
    let permissions = original_metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if permissions.mode() & 0o7000 != 0 {
            return Err(format!(
                "refusing atomic replacement of file with special Unix permission bits: {}",
                path.display()
            )
            .into());
        }
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    #[cfg(target_os = "linux")]
    let parent_handle = pinned_directory::open_parent(parent)?;
    #[cfg(target_os = "linux")]
    pinned_directory::try_lock_directory(&parent_handle)?;
    #[cfg(all(unix, not(target_os = "linux")))]
    let parent_handle = fs::File::open(parent)?;
    #[cfg(unix)]
    let parent_metadata = parent_handle.metadata()?;
    #[cfg(unix)]
    check_parent_identity(parent, &parent_metadata)?;
    let name = path.file_name().ok_or("write target has no filename")?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();

    for attempt in 0..128u32 {
        let temp_path = parent.join(format!(
            ".{}.oresfmt-{}-{nonce}-{attempt}.tmp",
            name.to_string_lossy(),
            std::process::id()
        ));
        #[cfg(target_os = "linux")]
        let file = pinned_directory::stage(
            &parent_handle,
            temp_path.file_name().ok_or("staging file missing basename")?,
        );
        #[cfg(not(target_os = "linux"))]
        let file = {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(&temp_path)
        };
        let mut file = match file {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        };
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            // Refuse inherited ACLs before exposing any staged source bytes.
            #[cfg(target_os = "linux")]
            pinned_directory::ensure_no_extended_attributes(&file)?;
            file.write_all(formatted.as_bytes())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let staged = file.metadata()?;
                if original_metadata.uid() != staged.uid()
                    || original_metadata.gid() != staged.gid()
                {
                    return Err(format!(
                        "refusing atomic replacement that changes owner/group: {}",
                        path.display()
                    )
                    .into());
                }
            }
            file.set_permissions(permissions.clone())?;
            #[cfg(target_os = "linux")]
            pinned_directory::ensure_no_extended_attributes(&file)?;
            file.sync_all()?;
            drop(file);
            // Recheck immediately before replacing the directory entry.
            revalidate_write_target(path, original)?;
            #[cfg(unix)]
            check_source_identity(path, initial_metadata)?;
            reject_unsupported_file_metadata(path)?;
            #[cfg(unix)]
            check_parent_identity(parent, &parent_metadata)?;
            #[cfg(target_os = "linux")]
            {
                pinned_directory::confirm_source(&parent_handle, name, initial_metadata, original)?;
                pinned_directory::replace(
                    &parent_handle,
                    temp_path.file_name().ok_or("staging file missing basename")?,
                    name,
                )?;
            }
            #[cfg(not(target_os = "linux"))]
            fs::rename(&temp_path, path)?;
            // Ensure the directory entry update is durable as well as the
            // already-synced file contents. A failed sync is reported, but
            // an already completed rename cannot be rolled back atomically.
            #[cfg(unix)]
            parent_handle.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            #[cfg(target_os = "linux")]
            let _ = pinned_directory::cleanup(
                &parent_handle,
                temp_path.file_name().ok_or("staging file missing basename")?,
            );
            #[cfg(not(target_os = "linux"))]
            let _ = fs::remove_file(&temp_path);
        }
        return result;
    }
    Err(format!("unable to allocate unique temporary file for {}", path.display()).into())
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

    let current = read_regular_source(path)?;
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

fn collect_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for path in inputs {
        collect_path(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    // Paths such as "a.ores" and "./a.ores" are lexically different but
    // designate the same target. Do not rewrite them twice: a second rewrite
    // would otherwise fail the expected-content revalidation after the first.
    let mut canonical = std::collections::HashSet::new();
    let mut unique = Vec::with_capacity(files.len());
    for path in files {
        if canonical.insert(fs::canonicalize(&path)?) {
            unique.push(path);
        }
    }
    let files = unique;
    if files.is_empty() {
        return Err("no .ores files found".into());
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
        } else if file_type.is_file() && child.extension().is_some_and(|ext| ext == "ores") {
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
    fn bounded_source_rejects_expanding_input() {
        let mut oversized = io::Cursor::new(vec![b'a'; MAX_SOURCE_BYTES as usize + 1]);
        let error = read_bounded_source(&mut oversized, "test source").unwrap_err();
        assert!(error.to_string().contains("refusing source exceeding"));

        let mut valid = io::Cursor::new(b"hello".to_vec());
        assert_eq!(read_bounded_source(&mut valid, "small").unwrap(), "hello");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cooperating_writes_refuse_concurrent_directory_lock() {
        let root = scratch("advisory-parent-lock");
        let first = pinned_directory::open_parent(&root).unwrap();
        pinned_directory::try_lock_directory(&first).unwrap();

        let second = pinned_directory::open_parent(&root).unwrap();
        let error = pinned_directory::try_lock_directory(&second).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(11)); // EWOULDBLOCK on Linux

        drop(first);
        pinned_directory::try_lock_directory(&second).unwrap();
        drop(second);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn staged_file_with_xattr_is_refused_before_rename() {
        use std::ffi::CString;
        use std::os::fd::AsRawFd;
        use std::os::raw::{c_char, c_int};
        use std::ffi::c_void;

        unsafe extern "C" {
            fn fsetxattr(
                fd: c_int,
                name: *const c_char,
                value: *const c_void,
                size: usize,
                flags: c_int,
            ) -> c_int;
        }

        let root = scratch("staged-xattr");
        let path = root.join("stage.tmp");
        let stage = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let name = CString::new("user.oresfmt.staged-security").unwrap();
        let value = b"unexpected";
        let status = unsafe {
            fsetxattr(
                stage.as_raw_fd(),
                name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
                0,
            )
        };
        assert_eq!(status, 0, "xattrs unavailable: {}", io::Error::last_os_error());

        let error = pinned_directory::ensure_no_extended_attributes(&stage).unwrap_err();
        assert!(error.to_string().contains("inherited extended attributes or ACLs"));
        assert!(fs::read_to_string(&path).unwrap().is_empty());

        let _ = fs::remove_dir_all(root);
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
    #[cfg(unix)]
    #[test]
    fn inode_guard_rejects_replacement_with_identical_bytes() {
        let root = scratch("same-bytes-different-inode");
        let path = root.join("sample.ores");
        let moved = root.join("original.ores");
        fs::write(&path, "before").unwrap();
        let original = fs::symlink_metadata(&path).unwrap();

        fs::rename(&path, &moved).unwrap();
        fs::write(&path, "before").unwrap();

        let err = check_source_identity(&path, &original).unwrap_err();
        assert!(err.to_string().contains("inode or metadata changed"));

        let _ = fs::remove_dir_all(root);
    }




    #[cfg(target_os = "linux")]
    #[test]
    fn source_read_rejects_fifo_without_waiting_for_a_writer() {
        let root = scratch("fifo-substitution");
        let pipe = root.join("source.ores");
        let status = Command::new("mkfifo").arg(&pipe).status().unwrap();
        assert!(status.success());

        let error = read_regular_source(&pipe).unwrap_err();
        assert!(
            error.to_string().contains("non-regular Oreslang source"),
            "{error}"
        );
        let error = reject_unsupported_file_metadata(&pipe).unwrap_err();
        assert!(error.to_string().contains("source path replaced"));
        let _ = fs::remove_dir_all(root);
    }


    #[cfg(target_os = "linux")]
    #[test]
    fn pinned_rename_never_follows_a_leaf_symlink_swapped_after_validation() {
        use std::ffi::OsStr;
        use std::os::unix::fs::symlink;

        let root = scratch("rename-leaf-symlink");
        let target = root.join("sample.ores");
        let outside = root.join("outside.ores");
        fs::write(&target, "before").unwrap();
        fs::write(&outside, "outside-unchanged").unwrap();

        let original = fs::symlink_metadata(&target).unwrap();
        let parent = pinned_directory::open_parent(&root).unwrap();
        pinned_directory::confirm_source(
            &parent,
            OsStr::new("sample.ores"),
            &original,
            "before",
        ).unwrap();

        let mut staged = pinned_directory::stage(&parent, OsStr::new("stage.tmp")).unwrap();
        staged.write_all(b"formatted").unwrap();
        drop(staged);

        fs::rename(&target, root.join("old-sample.ores")).unwrap();
        symlink(&outside, &target).unwrap();

        pinned_directory::replace(
            &parent,
            OsStr::new("stage.tmp"),
            OsStr::new("sample.ores"),
        ).unwrap();
        assert_eq!(fs::read_to_string(&outside).unwrap(), "outside-unchanged");
        assert_eq!(fs::read_to_string(&target).unwrap(), "formatted");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn pinned_source_inspection_refuses_symlink_leaf_substitution() {
        use std::ffi::OsStr;
        use std::os::unix::fs::symlink;

        let root = scratch("pinned-leaf-substitution");
        let target = root.join("sample.ores");
        let other = root.join("external.ores");
        let displaced = root.join("original.ores");
        fs::write(&target, "before").unwrap();
        fs::write(&other, "external-unmodified").unwrap();

        let expected = fs::symlink_metadata(&target).unwrap();
        let parent = pinned_directory::open_parent(&root).unwrap();
        fs::rename(&target, &displaced).unwrap();
        symlink(&other, &target).unwrap();

        let result = pinned_directory::confirm_source(
            &parent,
            OsStr::new("sample.ores"),
            &expected,
            "before",
        );
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&other).unwrap(), "external-unmodified");
        assert_eq!(fs::read_to_string(&displaced).unwrap(), "before");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn pinned_directory_handles_are_not_redirected_by_directory_replacement() {
        use std::ffi::OsStr;

        let root = scratch("pinned-dir-swap");
        let active = root.join("active");
        let moved = root.join("moved");
        let replacement = root.join("replacement");
        fs::create_dir(&active).unwrap();
        fs::create_dir(&replacement).unwrap();
        fs::write(active.join("sample.ores"), "original").unwrap();
        fs::write(replacement.join("sample.ores"), "untouched").unwrap();

        let handle = pinned_directory::open_parent(&active).unwrap();
        let identity = handle.metadata().unwrap();
        fs::rename(&active, &moved).unwrap();
        fs::rename(&replacement, &active).unwrap();

        let changed = check_parent_identity(&active, &identity).unwrap_err();
        assert!(changed.to_string().contains("directory replaced"));

        let mut staged = pinned_directory::stage(&handle, OsStr::new("stage.tmp")).unwrap();
        staged.write_all(b"new contents").unwrap();
        staged.sync_all().unwrap();
        drop(staged);

        assert!(moved.join("stage.tmp").exists());
        assert!(!active.join("stage.tmp").exists());

        pinned_directory::replace(
            &handle,
            OsStr::new("stage.tmp"),
            OsStr::new("sample.ores"),
        ).unwrap();
        assert_eq!(fs::read_to_string(moved.join("sample.ores")).unwrap(), "new contents");
        assert_eq!(fs::read_to_string(active.join("sample.ores")).unwrap(), "untouched");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn parent_guard_rejects_replaced_directory() {
        let root = scratch("replaced-parent");
        let parent = root.join("src");
        let displaced = root.join("original-src");
        fs::create_dir(&parent).unwrap();
        let original = fs::symlink_metadata(&parent).unwrap();

        fs::rename(&parent, &displaced).unwrap();
        fs::create_dir(&parent).unwrap();

        let err = check_parent_identity(&parent, &original).unwrap_err();
        assert!(err.to_string().contains("directory replaced"));

        let _ = fs::remove_dir_all(root);
    }

}
