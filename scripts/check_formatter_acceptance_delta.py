#!/usr/bin/env python3
"""Corpus differential: does formatting change what real compilers accept?

For every .atto/.ores file under the given corpus roots this harness:

1. formats the file twice and requires a successful, idempotent result;
2. runs each supplied compiler command on the original and on the formatted
   copy (both trees keep their relative layout, so relative imports resolve);
3. reports any change in acceptance.

Findings (each makes the run fail):

  broken         a compiler accepted the original but rejects the formatted copy
  newly-accepted a compiler rejected the original but accepts the formatted copy
                 (formatting changed the program's meaning, e.g. dropped `val`)
  rejects-valid  the formatter rejects a file at least one compiler accepts
  not-idempotent formatting the formatted copy changes it again

Acceptance means exit status 0. A compiler timeout or a missing tool is a
harness failure (exit 2), never silently counted as a rejection. This is
parse/typecheck admission evidence only: it is NOT guest execution parity and
NOT semantic AST equivalence (see check_compiler_ast_parity.py).

Example:

  scripts/check_formatter_acceptance_delta.py \\
    --formatter target/release/oresfmt \\
    --compiler 'java=java -cp "$CP" dev.oreslang.launcher.OresMain --check {source}' \\
    --compiler 'llvm=build/oreslang-llvmc {source}' \\
    --corpus ../oreslang-source.java --corpus ../oreslang-source.llvm
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import stat
import sys
import tempfile

from check_compiler_parity import command_argv, run_process

SOURCE_SUFFIXES = (".atto", ".ores")
SKIP_DIRS = {".git", "target", "build", "node_modules"}
MAX_SOURCE_BYTES = 16 * 1024 * 1024
MAX_SNAPSHOT_BYTES = 512 * 1024 * 1024
MAX_SNAPSHOT_FILES = 30000
MAX_FORMATTED_BYTES = 4 * 1024 * 1024

def snapshot_corpus(root, destination, integrity=None):
    """Copy all bounded regular repository inputs, not just source extensions.

    A source-only copy drops .ores.pkg.toml and auxiliary import resources,
    making both compiler admissions fail and falsely reporting no regression.
    Refuse links/special files and unstable inputs instead of silently
    following paths outside the source tree. Git/build caches are excluded.
    """
    if root.is_symlink():
        raise ValueError(f"corpus root is a symlink: {root}")
    total_bytes = 0
    total_files = 0
    for current, dirs, files in os.walk(root, followlinks=False):
        here = Path(current)
        dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
        for dirname in dirs:
            child = here / dirname
            if child.is_symlink():
                raise ValueError(f"symlinked corpus directory: {child}")
        out_dir = destination / here.relative_to(root)
        out_dir.mkdir(parents=True, exist_ok=True)
        for name in sorted(files):
            if name in SKIP_DIRS:
                continue
            source = here / name
            before = source.lstat()
            if not stat.S_ISREG(before.st_mode):
                raise ValueError(f"nonregular corpus input: {source}")
            if before.st_size > MAX_SOURCE_BYTES:
                raise ValueError(f"oversized corpus input: {source}")
            total_files += 1
            total_bytes += before.st_size
            if total_files > MAX_SNAPSHOT_FILES or total_bytes > MAX_SNAPSHOT_BYTES:
                raise ValueError(f"corpus snapshot exceeds bounded input budget: {root}")
            flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
            with os.fdopen(os.open(source, flags), "rb") as inp:
                opened = os.fstat(inp.fileno())
                identity = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_size,
                                      s.st_mtime_ns, s.st_ctime_ns)
                if identity(opened) != identity(before):
                    raise ValueError(f"unstable corpus input: {source}")
                dest = out_dir / name
                digest = hashlib.sha256()
                with dest.open("wb") as output:
                    while True:
                        chunk = inp.read(1024 * 1024)
                        if not chunk:
                            break
                        output.write(chunk)
                        digest.update(chunk)
                if integrity is not None:
                    integrity[dest] = digest.digest()
                if identity(os.fstat(inp.fileno())) != identity(opened) or (
                    identity(source.lstat()) != identity(opened)
                ):
                    raise ValueError(f"corpus input changed during snapshot: {source}")
                dest.chmod(stat.S_IMODE(before.st_mode) | stat.S_IWUSR)
    return total_files


def discover(roots):
    """Yield (root, relative path) for regular source files, never symlinks."""
    for root in roots:
        root = Path(root)
        for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
            dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
            for name in sorted(filenames):
                path = Path(dirpath) / name
                if path.suffix in SOURCE_SUFFIXES and path.is_file() and not path.is_symlink():
                    yield root, path.relative_to(root)


def format_file(formatter, path, timeout):
    # --stdout must never mutate the source. Otherwise a formatter that
    # rewrites the *original* before admission can conceal a regression:
    # both sides of the differential would be compared after mutation.
    try:
        before_bytes = path.read_bytes()
    except OSError as error:
        return "read-failure", None, str(error).encode()
    status, returncode, output, diagnostics = run_process(
        formatter + ["--stdout", str(path)], timeout, stdout_limit=MAX_FORMATTED_BYTES)
    try:
        if path.read_bytes() != before_bytes:
            return "mutated-input", None, b"formatter modified source under --stdout"
    except OSError as error:
        return "read-failure", None, str(error).encode()
    if status != "exited":
        return status, None, diagnostics
    if returncode != 0:
        return "rejected", None, diagnostics
    return "ok", output, diagnostics


def admit(template, path, timeout):
    status, returncode, _, diagnostics = run_process(command_argv(template, path), timeout)
    if status != "exited":
        return status, diagnostics
    if returncode is None or returncode < 0:
        return "crashed", diagnostics
    return ("accept" if returncode == 0 else "reject"), diagnostics


def audit_one(case, formatter, compilers, work, timeout):
    root, rel = case
    label = f"{root.name}/{rel}"
    original = work / "orig" / root.name / rel
    formatted = work / "fmt" / root.name / rel
    result = {"case": label, "findings": [], "harness_errors": []}

    first_status, first, diag = format_file(formatter, original, timeout)
    result["format"] = first_status
    if first_status not in ("ok", "rejected"):
        result["harness_errors"].append(f"formatter {first_status}: {diag[:300]!r}")
        return result
    if first is not None:
        formatted.write_bytes(first)
        second_status, second, diag = format_file(formatter, formatted, timeout)
        if second_status != "ok" or second != first:
            result["findings"].append("not-idempotent")

    result["compilers"] = {}
    baseline_admitted = False
    for name, template in compilers:
        before, diag = admit(template, original, timeout)
        if before not in ("accept", "reject"):
            result["harness_errors"].append(f"{name} {before} on original: {diag[:300]!r}")
            continue
        entry = {"original": before}
        baseline_admitted |= before == "accept"
        if first is None:
            if before == "accept":
                result["findings"].append(f"rejects-valid:{name}")
        else:
            after, diag = admit(template, formatted, timeout)
            if after not in ("accept", "reject"):
                result["harness_errors"].append(f"{name} {after} on formatted: {diag[:300]!r}")
                continue
            entry["formatted"] = after
            if before == "accept" and after == "reject":
                result["findings"].append(f"broken:{name}")
                entry["diagnostic"] = diag[:500].decode("utf-8", errors="replace")
            elif before == "reject" and after == "accept":
                result["findings"].append(f"newly-accepted:{name}")
        result["compilers"][name] = entry
    result["baseline_admitted"] = baseline_admitted
    return result


def parse_compiler(spec):
    name, sep, template = spec.partition("=")
    if not sep or not name or not template:
        raise argparse.ArgumentTypeError("expected NAME=COMMAND containing {source}")
    command_argv(template, "probe")  # validates {source}
    return name, template


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--formatter", required=True,
                        help="formatter command; invoked as FORMATTER --stdout FILE")
    parser.add_argument("--compiler", action="append", type=parse_compiler, default=[],
                        help="NAME=COMMAND with {source}; exit 0 means accepted")
    parser.add_argument("--corpus", action="append", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=60)
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    parser.add_argument("--report", type=Path, help="write the full JSON report here")
    args = parser.parse_args(argv)

    formatter = shlex.split(args.formatter)
    if not formatter:
        parser.error("empty formatter command")
    if not args.compiler:
        parser.error("at least one --compiler is required to claim a differential audit")
    compiler_names = [name for name, _ in args.compiler]
    if len(set(compiler_names)) != len(compiler_names):
        parser.error("compiler labels must be unique; duplicate names overwrite evidence")
    for root in args.corpus:
        if not root.is_dir():
            parser.error(f"corpus is not a directory: {root}")
    if len({root.name for root in args.corpus}) != len(args.corpus):
        parser.error("corpus directory names must be distinct")

    cases = list(discover(args.corpus))
    if not cases:
        print("HARNESS-ERROR no source cases discovered", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory(prefix="oresfmt-delta-") as tmp:
        work = Path(tmp)
        integrity = {}
        try:
            for root in args.corpus:
                original = work / "orig" / root.name
                snapshot_corpus(root, original, integrity)
                shutil.copytree(original, work / "fmt" / root.name)
        except (OSError, ValueError) as error:
            print(f"HARNESS-ERROR snapshot rejected: {error}", file=sys.stderr)
            return 2
        with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
            results = list(pool.map(
                lambda case: audit_one(case, formatter, args.compiler, work, args.timeout),
                cases))
        # Formatting must not modify ANY file in the original snapshot, not
        # just the input path. Cross-file source or package-manifest edits
        # could otherwise make both compiler admissions compare altered data.
        integrity_errors = []
        for path, digest in integrity.items():
            try:
                with path.open("rb") as source:
                    observed = hashlib.sha256()
                    while True:
                        chunk = source.read(1024 * 1024)
                        if not chunk:
                            break
                        observed.update(chunk)
                    if observed.digest() != digest:
                        integrity_errors.append(
                            ("<snapshot>", f"original input modified during audit: {path}")
                        )
            except OSError as error:
                integrity_errors.append(
                    ("<snapshot>", f"original input disappeared during audit: {path}: {error}")
                )
        # Run the negative control inside a package that THIS compiler already
        # admitted. A control outside the package could be rejected simply
        # for a missing .ores.pkg.toml, falsely certifying a syntax-blind stub.
        invalid_controls = []
        for name, template in args.compiler:
            first = next(
                (work / "orig" / root.name / rel
                 for (root, rel), row in zip(cases, results)
                 if row.get("compilers", {}).get(name, {}).get("original") == "accept"),
                None,
            )
            if first is None:
                continue  # the no-admitted-baseline guard handles this case
            probe = first.parent / "__oresfmt_invalid_syntax_control.atto"
            try:
                with probe.open("xb") as output:
                    output.write(b"fnc broken( !!!\\n")
                try:
                    control_status, diagnostics = admit(template, probe, args.timeout)
                finally:
                    probe.unlink()
            except OSError as error:
                invalid_controls.append(
                    ("<control>", f"{name}: unable to create/remove syntax control: {error}")
                )
                continue
            if control_status != "reject":
                invalid_controls.append((
                    "<control>", f"{name}: invalid syntax control was {control_status} "
                    f"(expected rejection within an admitted package): {diagnostics[:300]!r}"
                ))

    findings = [(r["case"], f) for r in results for f in r["findings"]]
    errors = [(r["case"], e) for r in results for e in r["harness_errors"]]
    errors.extend(invalid_controls)
    errors.extend(integrity_errors)
    admitted = {name: sum(r.get("compilers", {}).get(name, {}).get("original") == "accept"
                          for r in results) for name, _ in args.compiler}
    for name, count in admitted.items():
        if count == 0:
            errors.append(("<corpus>", f"{name}: no original sources accepted; all reject/reject is not parity evidence"))
    summary = {
        "schema": "oreslang-format-acceptance-delta-v1",
        "files": len(results),
        "formatter_rejected": sum(r.get("format") == "rejected" for r in results),
        "compilers": [name for name, _ in args.compiler],
        "baseline_accepted_by_compiler": admitted,
        "baseline_unadmitted": sum(not r.get("baseline_admitted", False) for r in results),
        "findings": len(findings),
        "harness_errors": len(errors),
    }
    if args.report:
        args.report.write_text(json.dumps({"summary": summary, "results": results}, indent=2) + "\n",
                               encoding="utf-8")
    for case, finding in findings:
        print(f"FINDING {finding}: {case}")
    for case, error in errors:
        print(f"HARNESS-ERROR {case}: {error}", file=sys.stderr)
    print(json.dumps(summary))
    if errors:
        return 2
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
