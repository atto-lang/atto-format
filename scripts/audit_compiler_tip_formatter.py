#!/usr/bin/env python3
"""Read-only, SHA-pinned oresfmt coverage gate over all five compiler tips.

This checks source discovery, formatting success and byte idempotence. It does
NOT certify compiler admission, AST equivalence, borrow soundness or guest AOT.
Use check_formatter_acceptance_delta.py (when integrated) with real compilers
for admission evidence; never infer that evidence from this gate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile

from check_compiler_parity import run_process

REPOS = (
    "oreslang-source.java",
    "oreslang-source.llvm",
    "oreslang-source.wasm",
    "oreslang-source.net",
    "oreslang-source.forge",
)
SUFFIXES = {".atto", ".ores"}
MAX_BYTES = 8 * 1024 * 1024
MAX_TRACKED = 20000
MAX_TRACKED_BYTES = 64 * 1024 * 1024


def parse_named(spec):
    name, sep, value = spec.partition("=")
    if not sep or name not in REPOS or not value:
        raise argparse.ArgumentTypeError("expected oreslang-source.BACKEND=VALUE")
    return name, value


def git(repo, *args):
    # Inherited Git control variables can redirect -C to a different
    # repository, object database, index or remote config. Never use them
    # as an authority for compiler source identity.
    clean_env = {key: value for key, value in os.environ.items()
                 if not key.startswith("GIT_")}
    # Git replace refs stored inside .git can transparently substitute a
    # different commit tree while rev-parse still prints the trusted HEAD.
    # Ignore all replacement objects, even those created locally.
    clean_env["GIT_NO_REPLACE_OBJECTS"] = "1"
    clean_env["LC_ALL"] = "C"
    result = subprocess.run(
        ["git", "-c", "core.fsmonitor=false",
         "-c", "core.hooksPath=/dev/null",
         "-C", str(repo), *args], stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False, timeout=30, env=clean_env,
    )
    if result.returncode:
        raise ValueError(f"git {' '.join(args)} failed in {repo}: "
                         f"{result.stderr[:300]!r}")
    return result.stdout


def inspect_repo(name, repo, sha):
    if repo.is_symlink() or not repo.is_dir():
        raise ValueError(f"{name}: missing or symlinked worktree: {repo}")
    expected_root = repo.resolve(strict=True)
    reported_root = git(repo, "rev-parse", "--show-toplevel").decode("utf-8").strip()
    if Path(reported_root).resolve(strict=True) != expected_root:
        raise ValueError(f"{name}: not an exact Git repository root: {repo}")
    actual = git(repo, "rev-parse", "HEAD").decode("ascii").strip()
    if actual != sha:
        raise ValueError(f"{name}: stale checkout: HEAD {actual}, expected {sha}")
    if git(repo, "status", "--porcelain=v1", "--untracked-files=no"):
        raise ValueError(f"{name}: tracked worktree is dirty")

    paths = [raw for raw in git(repo, "ls-files", "-z").split(b"\0") if raw]
    if len(paths) > MAX_TRACKED:
        raise ValueError(f"{name}: tracked-file limit exceeded")
    object_format = git(repo, "rev-parse", "--show-object-format=storage").decode("ascii").strip()
    if object_format not in ("sha1", "sha256"):
        raise ValueError(f"{name}: unsupported Git object hash: {object_format}")

    # Include every tracked file, not just AttoLang source. An attacker can
    # hide edits to package manifests, compiler config, or imported resources
    # with assume-unchanged / skip-worktree flags.
    expected_blobs = {}
    for entry in git(repo, "ls-tree", "-r", "-z", "HEAD").split(b"\0"):
        if not entry:
            continue
        metadata, separator, raw_path = entry.partition(b"\t")
        parts = metadata.split()
        if not separator or len(parts) != 3:
            raise ValueError(f"{name}: malformed immutable Git tree entry")
        mode, kind, blob_sha = parts
        try:
            rel = Path(raw_path.decode("utf-8"))
        except UnicodeDecodeError as error:
            raise ValueError(f"{name}: non-UTF8 Git tree path") from error
        if rel.is_absolute() or ".." in rel.parts or rel in expected_blobs:
            raise ValueError(f"{name}: unsafe Git tree path: {rel}")
        if kind != b"blob" or mode not in (b"100644", b"100755"):
            raise ValueError(f"{name}: unsafe Git tree entry: {rel}")
        expected_blobs[rel] = (blob_sha.decode("ascii"), mode)

    source_paths = []
    seen = set()
    total_bytes = 0
    for raw in paths:
        try:
            rel = Path(raw.decode("utf-8"))
        except UnicodeDecodeError as error:
            raise ValueError(f"{name}: non-UTF8 tracked filename") from error
        if rel.is_absolute() or ".." in rel.parts or rel in seen:
            raise ValueError(f"{name}: unsafe tracked path: {rel}")
        seen.add(rel)
        if rel not in expected_blobs:
            raise ValueError(f"{name}: index file absent from immutable HEAD tree: {rel}")
        path = repo / rel
        for part in (repo / Path(*rel.parts[:i]) for i in range(1, len(rel.parts) + 1)):
            if part.is_symlink():
                raise ValueError(f"{name}: linked source component: {part}")
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_BYTES:
            raise ValueError(f"{name}: unsafe or oversized tracked file: {rel}")
        if os.name == "posix":
            expected_executable = expected_blobs[rel][1] == b"100755"
            if bool(info.st_mode & 0o111) != expected_executable:
                raise ValueError(f"{name}: physical executable mode mismatch: {rel}")
        flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
        flags |= getattr(os, "O_NONBLOCK", 0)
        with os.fdopen(os.open(path, flags), "rb") as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise ValueError(f"{name}: tracked file replaced by special file: {rel}")
            physical = stream.read(MAX_BYTES + 1)
        if len(physical) > MAX_BYTES:
            raise ValueError(f"{name}: tracked file grew past bounded read: {rel}")
        total_bytes += len(physical)
        if total_bytes > MAX_TRACKED_BYTES:
            raise ValueError(f"{name}: total tracked byte budget exceeded")
        payload = b"blob " + str(len(physical)).encode("ascii") + b"\0" + physical
        actual_blob_sha = hashlib.new(object_format, payload).hexdigest()
        if actual_blob_sha != expected_blobs[rel][0]:
            raise ValueError(f"{name}: physical Git blob mismatch: {rel}")
        if rel.suffix in SUFFIXES:
            source_paths.append((rel, path))

    if seen != set(expected_blobs):
        raise ValueError(f"{name}: index file list does not match immutable HEAD tree")
    if not source_paths:
        raise ValueError(f"{name}: no tracked .atto/.ores source at {sha}")
    return source_paths

def format_once(formatter, source, timeout):
    status, code, stdout, stderr = run_process(
        [str(formatter), "--stdout", str(source)], timeout,
        stdout_limit=2 * MAX_BYTES,
    )
    if status != "exited" or code != 0:
        raise ValueError(
            f"formatter {status} / exit {code}: "
            f"{stderr[:250].decode('utf-8', errors='replace')}"
        )
    try:
        stdout.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError("formatter emitted non-UTF8 output") from error
    return stdout


def audit_source(formatter, source, suffix, timeout):
    # Bound the read even if a checked source grows after the tracked-file
    # inventory. O_NOFOLLOW also closes a leaf-symlink substitution race.
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    flags |= getattr(os, "O_NONBLOCK", 0)
    with os.fdopen(os.open(source, flags), "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("source is not a regular file")
        before = stream.read(MAX_BYTES + 1)
    if len(before) > MAX_BYTES:
        raise ValueError("source exceeded byte bound during read")
    try:
        before.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError("non-UTF8 compiler source") from error
    original_sha256 = hashlib.sha256(before).hexdigest()
    with tempfile.TemporaryDirectory(prefix="oresfmt-tip-") as tmp:
        once = format_once(formatter, source, timeout)
        again_path = Path(tmp) / ("again" + suffix)
        again_path.write_bytes(once)
        twice = format_once(formatter, again_path, timeout)
    if once != twice:
        raise ValueError("formatter output is not idempotent")
    if source.read_bytes() != before:
        raise ValueError("formatter mutated input despite --stdout")
    return {"changed": once != before, "sha256": original_sha256}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--formatter", type=Path, required=True)
    parser.add_argument("--repo", action="append", type=parse_named, required=True,
                        help="oreslang-source.BACKEND=/local/checked-out/worktree")
    parser.add_argument("--head", action="append", type=parse_named, required=True,
                        help="oreslang-source.BACKEND=EXACT_40_HEX_COMMIT")
    parser.add_argument("--timeout", type=int, default=30)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args(argv)
    repos, heads = dict(args.repo), dict(args.head)
    if (len(args.repo) != len(REPOS) or len(args.head) != len(REPOS)
            or set(repos) != set(REPOS) or set(heads) != set(REPOS)):
        parser.error("exactly one --repo and --head for each of the five backends is required")
    if args.timeout < 1 or args.timeout > 300:
        parser.error("--timeout must be 1..300 seconds")
    for name, sha in heads.items():
        if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha):
            parser.error(f"{name}: --head must be a lowercase 40-character commit SHA")
    formatter = args.formatter.resolve()
    if not formatter.is_file() or not os.access(formatter, os.X_OK):
        parser.error("formatter must be an existing executable")

    formatter_digest = hashlib.sha256()
    with formatter.open("rb") as binary:
        for chunk in iter(lambda: binary.read(1024 * 1024), b""):
            formatter_digest.update(chunk)
    report = {"schema": "oresfmt-compiler-tip-v1", "ok": False,
              "scope": "read-only-formatting-and-idempotence-only",
              "formatter_sha256": formatter_digest.hexdigest(),
              "repos": {}, "failures": []}
    for name in REPOS:
        supplied_root = Path(repos[name])
        root = supplied_root.resolve()
        stats = {"head": heads[name], "files": 0, "changed": 0, "unchanged": 0}
        report["repos"][name] = stats
        try:
            if supplied_root.is_symlink():
                raise ValueError("symlinked worktree argument")
            files = inspect_repo(name, root, heads[name])
            stats["tree"] = git(root, "rev-parse", "HEAD^{tree}").decode("ascii").strip()
            for rel, source in files:
                stats["files"] += 1
                try:
                    result = audit_source(formatter, source, rel.suffix, args.timeout)
                    stats["changed" if result["changed"] else "unchanged"] += 1
                except (OSError, ValueError) as error:
                    report["failures"].append(f"{name}/{rel}: {error}")
            # A formatter processing a later file can alter an earlier one,
            # even with assume-unchanged set. Recheck every immutable blob
            # after the final invocation; a preflight alone is insufficient.
            verified = inspect_repo(name, root, heads[name])
            if [rel for rel, _ in verified] != [rel for rel, _ in files]:
                report["failures"].append(f"{name}: source inventory changed during audit")
        except (OSError, ValueError, subprocess.TimeoutExpired) as error:
            report["failures"].append(f"{name}: {error}")
    # The formatter binary is an executable input to every result. Reject a
    # replacement/modification during the audit rather than recording the
    # hash of a different executable than the one used.
    try:
        final_formatter_digest = hashlib.sha256()
        with formatter.open("rb") as binary:
            for chunk in iter(lambda: binary.read(1024 * 1024), b""):
                final_formatter_digest.update(chunk)
        if final_formatter_digest.hexdigest() != report["formatter_sha256"]:
            report["failures"].append("formatter executable modified during audit")
    except OSError as error:
        report["failures"].append(f"formatter executable unavailable after audit: {error}")
    report["ok"] = not report["failures"] and all(
        stats["files"] > 0 for stats in report["repos"].values()
    )
    if args.report:
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n",
                               encoding="utf-8")
    for name, stats in report["repos"].items():
        print(f"{name} @{stats['head'][:12]}: {stats['files']} tracked sources, "
              f"{stats['changed']} changed, {stats['unchanged']} already canonical")
    for failure in report["failures"][:30]:
        print(f"FAIL: {failure}", file=sys.stderr)
    if len(report["failures"]) > 30:
        print(f"... {len(report['failures']) - 30} further failures", file=sys.stderr)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
