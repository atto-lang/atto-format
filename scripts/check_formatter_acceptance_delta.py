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
import json
import os
from pathlib import Path
import shlex
import shutil
import sys
import tempfile

from check_compiler_parity import command_argv, run_process

SOURCE_SUFFIXES = (".atto", ".ores")
SKIP_DIRS = {".git", "target", "build", "node_modules"}
MAX_SOURCE_BYTES = 1024 * 1024
MAX_FORMATTED_BYTES = 4 * 1024 * 1024


def discover(roots):
    """Yield (root, relative path) for regular source files, never symlinks."""
    for root in roots:
        root = Path(root)
        for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
            dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
            for name in sorted(filenames):
                path = Path(dirpath) / name
                if path.suffix in SOURCE_SUFFIXES and path.is_file() and not path.is_symlink():
                    if path.stat().st_size <= MAX_SOURCE_BYTES:
                        yield root, path.relative_to(root)


def format_file(formatter, path, timeout):
    status, returncode, output, diagnostics = run_process(
        formatter + ["--stdout", str(path)], timeout, stdout_limit=MAX_FORMATTED_BYTES)
    if status != "exited":
        return status, None, diagnostics
    if returncode != 0:
        return "rejected", None, diagnostics
    return "ok", output, diagnostics


def admit(template, path, timeout):
    status, returncode, _, diagnostics = run_process(command_argv(template, path), timeout)
    if status != "exited":
        return status, diagnostics
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
    for name, template in compilers:
        before, diag = admit(template, original, timeout)
        if before not in ("accept", "reject"):
            result["harness_errors"].append(f"{name} {before} on original: {diag[:300]!r}")
            continue
        entry = {"original": before}
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
    for root in args.corpus:
        if not root.is_dir():
            parser.error(f"corpus is not a directory: {root}")
    if len({root.name for root in args.corpus}) != len(args.corpus):
        parser.error("corpus directory names must be distinct")

    cases = list(discover(args.corpus))
    with tempfile.TemporaryDirectory(prefix="oresfmt-delta-") as tmp:
        work = Path(tmp)
        for root, rel in cases:
            for tree in ("orig", "fmt"):
                (work / tree / root.name / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(root / rel, work / "orig" / root.name / rel)
        with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
            results = list(pool.map(
                lambda case: audit_one(case, formatter, args.compiler, work, args.timeout),
                cases))

    findings = [(r["case"], f) for r in results for f in r["findings"]]
    errors = [(r["case"], e) for r in results for e in r["harness_errors"]]
    summary = {
        "schema": "oreslang-format-acceptance-delta-v1",
        "files": len(results),
        "formatter_rejected": sum(r.get("format") == "rejected" for r in results),
        "compilers": [name for name, _ in args.compiler],
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
