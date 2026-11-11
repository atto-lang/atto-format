#!/usr/bin/env python3
"""Read-only, fail-closed CLI formatting audit of pinned local compiler heads.

No network access, private-source transfer, guest execution, or implicit
compiler invocation. Actual compiler AST/AOT parity is a separate gate.
"""
import argparse
import hashlib
import json
import os
import re
import shlex
import stat
import subprocess
import tempfile
from pathlib import Path

from check_compiler_parity import run_process

COMPILERS = frozenset({"java", "llvm", "net", "wasm", "forge", "rs", "js"})
MAX_BYTES_LIMIT = 8 * 1024 * 1024


def git(root, *arguments):
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        timeout=30,
    )
    if result.returncode:
        detail = result.stderr.decode("utf-8", errors="replace")[:300]
        raise ValueError(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout


def repo_argument(value):
    try:
        name_and_path, sha = value.rsplit("@", 1)
        name, location = name_and_path.split("=", 1)
    except ValueError as error:
        raise argparse.ArgumentTypeError(
            "--repo must be name=/local/repo@40-character-HEAD-sha"
        ) from error
    if name not in COMPILERS or not location or not re.fullmatch(r"[0-9a-fA-F]{40}", sha):
        raise argparse.ArgumentTypeError("unknown compiler, missing path or invalid full HEAD SHA")
    return name, Path(location).resolve(), sha.lower()


def checked_file(root, name, max_bytes):
    candidate = root / name
    if candidate.suffix not in {".atto", ".ores"}:
        raise ValueError("unexpected source extension")
    if candidate.is_symlink():
        raise ValueError("source is a symlink")
    if candidate.resolve() != candidate or not candidate.is_relative_to(root):
        raise ValueError("source escapes the checked-out repository or has symlink ancestors")
    # O_NOFOLLOW and O_NONBLOCK prevent a replaced symlink/FIFO from being followed.
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    fd = os.open(candidate, flags)
    with os.fdopen(fd, "rb") as source:
        metadata = os.fstat(source.fileno())
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > max_bytes:
            raise ValueError("source is not a bounded regular file")
        data = source.read(max_bytes + 1)
    if len(data) > max_bytes:
        raise ValueError("source exceeds byte limit")
    data.decode("utf-8")
    return data


def formatted_output(formatter, filename, timeout, max_bytes):
    status, code, output, diagnostic = run_process(
        formatter + ["--stdout", str(filename)],
        timeout, stdout_limit=max_bytes * 2,
    )
    if status != "exited" or code != 0:
        detail = diagnostic.decode("utf-8", errors="replace")[:300]
        raise ValueError(f"formatter {status}, exit={code}: {detail}")
    output.decode("utf-8")
    return output


def audit_file(root, name, formatter, timeout, max_bytes, require_clean):
    source = checked_file(root, name, max_bytes)
    with tempfile.TemporaryDirectory(prefix="oresfmt-tip-") as temporary:
        staged = Path(temporary).resolve() / "formatted.atto"
        first = formatted_output(formatter, root / name, timeout, max_bytes)
        if source.strip() and not first.strip():
            raise ValueError("formatter erased nonempty source")
        staged.write_bytes(first)
        second = formatted_output(formatter, staged, timeout, max_bytes)
    if first != second:
        raise ValueError("formatter is not idempotent")
    changed = source != first
    return {
        "path": name, "source_sha256": hashlib.sha256(source).hexdigest(),
        "formatted_sha256": hashlib.sha256(first).hexdigest(),
        "changed": changed, "status": "ok",
    }


def audit_repo(name, root, expected, formatter, timeout, max_bytes, max_files, require_clean):
    result = {
        "repo": name, "path": str(root), "expected_head": expected,
        "ok": False, "scanned": 0, "changed": 0, "failures": [],
    }
    try:
        if not root.is_dir():
            raise ValueError("repository directory is missing")
        actual = git(root, "rev-parse", "--verify", "HEAD").decode().strip()
        result["actual_head"] = actual
        result["tree"] = git(root, "rev-parse", "HEAD^{tree}").decode().strip()
        if actual != expected:
            raise ValueError("local compiler HEAD does not match independently pinned tip")
        if git(root, "status", "--porcelain=v1", "-z", "--untracked-files=no"):
            raise ValueError("compiler checkout has uncommitted tracked changes")
        all_names = git(root, "ls-files", "--cached", "-z").split(b"\0")
        names = sorted(
            os.fsdecode(entry) for entry in all_names if entry
            and Path(os.fsdecode(entry)).suffix in {".atto", ".ores"}
        )
        if not names:
            raise ValueError("no tracked .atto or .ores source found")
        if len(names) > max_files:
            raise ValueError(f"source count {len(names)} exceeds --max-files={max_files}")
        digest = hashlib.sha256()
        inventoried = 0
        for path in names:
            result["scanned"] += 1
            try:
                record = audit_file(root, path, formatter, timeout, max_bytes, require_clean)
                result["changed"] += int(record["changed"])
                if require_clean and record["changed"]:
                    result["failures"].append({
                        "path": path, "error": "compiler source differs from canonical formatting"
                    })
                # Pin each checked source path and its exact pre-format bytes.
                inventoried += 1
                digest.update(path.encode("utf-8", errors="surrogateescape"))
                digest.update(b"\0")
                digest.update(bytes.fromhex(record["source_sha256"]))
                digest.update(b"\n")
            except (OSError, ValueError, UnicodeError) as error:
                result["failures"].append({"path": path, "error": str(error)[:500]})
        result["source_manifest_sha256"] = digest.hexdigest()
        result["source_manifest_complete"] = inventoried == len(names)
        result["ok"] = not result["failures"] and result["source_manifest_complete"]
    except (OSError, ValueError, UnicodeError, subprocess.TimeoutExpired) as error:
        result["failures"].append({"error": str(error)[:500]})
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--formatter", required=True, help="reviewed oresfmt command (no shell)")
    parser.add_argument("--repo", action="append", required=True, type=repo_argument,
                        help="repeat: java=/local/oreslang-source.java@<40-char-HEAD-sha>")
    parser.add_argument("--require-all", action="store_true",
                        help="require java, llvm, net, wasm, forge and rs")
    parser.add_argument("--require-clean", action="store_true",
                        help="also fail if any compiler source would be changed")
    parser.add_argument("--timeout", type=float, default=10.0)
    parser.add_argument("--max-file-bytes", type=int, default=1024 * 1024)
    parser.add_argument("--max-files", type=int, default=2000)
    args = parser.parse_args(argv)
    if args.timeout <= 0 or not (0 < args.max_file_bytes <= MAX_BYTES_LIMIT) or args.max_files <= 0:
        parser.error("timeout, max-file-bytes (up to 8MiB) and max-files must be positive")
    try:
        formatter = shlex.split(args.formatter)
    except ValueError as error:
        parser.error(str(error))
    if not formatter:
        parser.error("formatter command must not be empty")
    names = [name for name, _, _ in args.repo]
    if len(set(names)) != len(names):
        parser.error("duplicate compiler names are forbidden")
    missing = sorted({"java", "llvm", "net", "wasm", "forge", "rs"} - set(names))
    if args.require_all and missing:
        parser.error(f"missing compiler repos: {', '.join(missing)}")
    results = [
        audit_repo(name, path, sha, formatter, args.timeout, args.max_file_bytes,
                   args.max_files, args.require_clean)
        for name, path, sha in args.repo
    ]
    ok = bool(results) and all(r["ok"] for r in results)
    print(json.dumps({
        "schema": "oresfmt-compiler-tip-audit-v1",
        "ok": ok, "require_clean": args.require_clean,
        "complete_six_backend_inventory": not missing,
        "missing_backends": missing, "results": results,
        "evidence_tier": "read-only formatter output and idempotence; not compiler AST/guest/AOT",
    }, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
