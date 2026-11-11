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


def parse_named(spec):
    name, sep, value = spec.partition("=")
    if not sep or name not in REPOS or not value:
        raise argparse.ArgumentTypeError("expected oreslang-source.BACKEND=VALUE")
    return name, value


def git(repo, *args):
    result = subprocess.run(
        ["git", "-C", str(repo), *args], stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False, timeout=30,
    )
    if result.returncode:
        raise ValueError(f"git {' '.join(args)} failed in {repo}: "
                         f"{result.stderr[:300]!r}")
    return result.stdout


def inspect_repo(name, repo, sha):
    if repo.is_symlink() or not repo.is_dir():
        raise ValueError(f"{name}: missing or symlinked worktree: {repo}")
    actual = git(repo, "rev-parse", "HEAD").decode("ascii").strip()
    if actual != sha:
        raise ValueError(f"{name}: stale checkout: HEAD {actual}, expected {sha}")
    if git(repo, "status", "--porcelain=v1", "--untracked-files=no"):
        raise ValueError(f"{name}: tracked worktree is dirty")
    paths = git(repo, "ls-files", "-z").split(b"\0")
    if len(paths) > MAX_TRACKED + 1:
        raise ValueError(f"{name}: tracked-file limit exceeded")
    source_paths = []
    for raw in paths:
        if not raw:
            continue
        try:
            rel = Path(raw.decode("utf-8"))
        except UnicodeDecodeError as error:
            raise ValueError(f"{name}: non-UTF8 tracked filename") from error
        if rel.suffix not in SUFFIXES:
            continue
        if rel.is_absolute() or ".." in rel.parts:
            raise ValueError(f"{name}: unsafe tracked path: {rel}")
        source = repo / rel
        # Reject tracked symlinks, including parent directories. Never
        # follow a linked source out of the approved compiler worktree.
        for part in (repo / Path(*rel.parts[:i]) for i in range(1, len(rel.parts) + 1)):
            if part.is_symlink():
                raise ValueError(f"{name}: linked source component: {part}")
        info = source.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_BYTES:
            raise ValueError(f"{name}: unsafe or oversized source: {rel}")
        source_paths.append((rel, source))
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

    report = {"schema": "oresfmt-compiler-tip-v1", "ok": False,
              "scope": "read-only-formatting-and-idempotence-only", "repos": {},
              "failures": []}
    for name in REPOS:
        supplied_root = Path(repos[name])
        root = supplied_root.resolve()
        stats = {"head": heads[name], "files": 0, "changed": 0, "unchanged": 0}
        report["repos"][name] = stats
        try:
            if supplied_root.is_symlink():
                raise ValueError("symlinked worktree argument")
            files = inspect_repo(name, root, heads[name])
            for rel, source in files:
                stats["files"] += 1
                try:
                    result = audit_source(formatter, source, rel.suffix, args.timeout)
                    stats["changed" if result["changed"] else "unchanged"] += 1
                except (OSError, ValueError) as error:
                    report["failures"].append(f"{name}/{rel}: {error}")
            if git(root, "rev-parse", "HEAD").decode("ascii").strip() != heads[name]:
                report["failures"].append(f"{name}: HEAD changed during audit")
            if git(root, "status", "--porcelain=v1", "--untracked-files=no"):
                report["failures"].append(f"{name}: tracked worktree changed during audit")
        except (OSError, ValueError, subprocess.TimeoutExpired) as error:
            report["failures"].append(f"{name}: {error}")
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
