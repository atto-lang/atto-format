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

Reviewed findings (for example a documented migration that repairs source a
compiler rejected) can be listed in a --triaged file, one per line:
`CASE<TAB>FINDING<TAB>reason`. Triaged findings are reported but do not fail
the run; a triaged entry that no longer occurs is reported as stale. Triage
never suppresses harness errors.

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
    # Refuse an existing symlink in any ancestor, not only at the final
    # corpus component. A directory under a linked parent is otherwise
    # traversed by os.walk even though root.is_symlink() itself is false.
    for component in (root, *root.parents):
        if component.is_symlink():
            raise ValueError(f"symlinked corpus root/ancestor: {component}")
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
    if returncode is None or returncode < 0:
        return "crashed", None, diagnostics
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


def prepare_one(case, formatter, work, timeout):
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

    return result


def compare_one(case, result, compilers, work, timeout):
    """Audit compiler admission only after every source has been formatted."""
    if result.get("format") not in ("ok", "rejected"):
        return result
    root, rel = case
    original = work / "orig" / root.name / rel
    formatted = work / "fmt" / root.name / rel
    result["compilers"] = {}
    baseline_admitted = False
    for name, template in compilers:
        before, diag = admit(template, original, timeout)
        if before not in ("accept", "reject"):
            result["harness_errors"].append(f"{name} {before} on original: {diag[:300]!r}")
            continue
        entry = {"original": before}
        baseline_admitted |= before == "accept"
        if result["format"] == "rejected":
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
    parser.add_argument("--triaged", type=Path,
                        help="CASE<TAB>FINDING<TAB>reason lines for reviewed findings")
    args = parser.parse_args(argv)
    triaged = {}
    if args.triaged:
        for number, line in enumerate(args.triaged.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip() or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) < 3 or not parts[2].strip():
                parser.error(f"{args.triaged}:{number}: expected CASE<TAB>FINDING<TAB>reason")
            triaged[(parts[0], parts[1])] = parts[2].strip()

    formatter = shlex.split(args.formatter)
    if not formatter:
        parser.error("empty formatter command")
    if not args.compiler:
        parser.error("at least one --compiler is required to claim a differential audit")
    compiler_names = [name for name, _ in args.compiler]
    if len(set(compiler_names)) != len(compiler_names):
        parser.error("compiler labels must be unique; duplicate names overwrite evidence")
    if args.jobs < 1 or args.timeout < 1:
        parser.error("--jobs and --timeout must be positive integers")
    resolved = []
    for root in args.corpus:
        if root.is_symlink():
            parser.error(f"corpus root is a symlink: {root}")
        if not root.is_dir():
            parser.error(f"corpus is not a directory: {root}")
        # Resolve operator-supplied ancestors once (macOS /tmp and /var are
        # system symlinks to /private). snapshot_corpus still rejects any
        # symlinked component of the resolved path, so a component swapped
        # during the audit fails closed.
        resolved.append(root.resolve(strict=True))
    args.corpus = resolved
    if len({root.name for root in args.corpus}) != len(args.corpus):
        parser.error("corpus directory names must be distinct")

    cases = list(discover(args.corpus))
    if not cases:
        print("HARNESS-ERROR no source cases discovered", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory(prefix="oresfmt-delta-") as tmp:
        work = Path(tmp)
        integrity = {}
        formatted_auxiliary = {}
        try:
            for root in args.corpus:
                original = work / "orig" / root.name
                formatted_root = work / "fmt" / root.name
                snapshot_corpus(root, original, integrity)
                shutil.copytree(original, formatted_root)
                for source, digest in integrity.items():
                    if source.is_relative_to(original):
                        rel = source.relative_to(original)
                        if rel.suffix not in SOURCE_SUFFIXES:
                            formatted_auxiliary[formatted_root / rel] = digest
        except (OSError, ValueError) as error:
            print(f"HARNESS-ERROR snapshot rejected: {error}", file=sys.stderr)
            return 2
        # Phase barrier: comparing a file while another thread is still
        # formatting its imported sibling observes a mixed project tree and
        # can create spurious acceptance differences. Finish all formatted
        # outputs before executing any compiler admission command.
        # Formatter subprocesses must be sequential: concurrent invocations
        # can rewrite another file before its own formatting pass records the
        # original content, making the audit schedule-dependent.
        prepared = [prepare_one(case, formatter, work, args.timeout)
                    for case in cases]
        # Capture the fully prepared source tree before any compiler runs.
        # A compiler that rewrites an imported module must never be able to
        # influence a later admission and still yield a passing receipt.
        formatted_source_integrity = {}
        for root, rel in cases:
            path = work / "fmt" / root.name / rel
            try:
                formatted_source_integrity[path] = hashlib.sha256(path.read_bytes()).digest()
            except OSError:
                # The per-case result or inventory check reports this too.
                formatted_source_integrity[path] = None
        with ThreadPoolExecutor(max_workers=args.jobs) as pool:
            results = list(pool.map(
                lambda pair: compare_one(pair[0], pair[1], args.compiler, work, args.timeout),
                zip(cases, prepared)))
        # Run the negative syntax controls before the final integrity scan.
        # The control compiler is untrusted to leave the corpus untouched;
        # any resulting manifest/source mutation must be visible in evidence.
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
        # Source files in the formatted copy are intentionally rewritten.
        # Manifests/resources are NOT: changing them can turn broken source
        # into a misleading compiler admission, even when original is intact.
        for path, digest in formatted_auxiliary.items():
            try:
                if path.is_symlink() or not path.is_file():
                    raise OSError("auxiliary input replaced with nonregular file")
                if hashlib.sha256(path.read_bytes()).digest() != digest:
                    raise ValueError("auxiliary content changed")
            except (OSError, ValueError) as error:
                integrity_errors.append(
                    ("<snapshot>", f"formatted auxiliary input modified during audit: {path}: {error}")
                )
        for path, digest in formatted_source_integrity.items():
            try:
                if path.is_symlink() or not path.is_file():
                    raise OSError("formatted source replaced with nonregular file")
                if digest is None or hashlib.sha256(path.read_bytes()).digest() != digest:
                    raise ValueError("formatted source content changed")
            except (OSError, ValueError) as error:
                integrity_errors.append(
                    ("<snapshot>", f"formatted source modified by compiler: {path}: {error}")
                )
        # A compiler/formatter must not add, remove or symlink ANY source
        # or auxiliary input in either tree. An added .atto file was
        # previously invisible because only auxiliary extensions were
        # scanned; a new import could silently change admission results.
        for root in args.corpus:
            original_root = work / "orig" / root.name
            formatted_root = work / "fmt" / root.name
            original_paths = {source for source in integrity
                              if source.is_relative_to(original_root)}
            for tree_name, tree_root, expected_paths in (
                ("original", original_root, original_paths),
                ("formatted", formatted_root, {
                    formatted_root / source.relative_to(original_root)
                    for source in original_paths
                }),
            ):
                for current, dirs, files in os.walk(tree_root, followlinks=False):
                    here = Path(current)
                    for name in dirs:
                        item = here / name
                        if item.is_symlink():
                            integrity_errors.append(
                                ("<snapshot>", f"{tree_name} symlink directory created during audit: {item}")
                            )
                    for name in files:
                        item = here / name
                        if item not in expected_paths or item.is_symlink():
                            integrity_errors.append(
                                ("<snapshot>", f"{tree_name} unexpected/replaced file during audit: {item}")
                            )
                for item in expected_paths:
                    if item.is_symlink() or not item.is_file():
                        integrity_errors.append(
                            ("<snapshot>", f"{tree_name} input removed/replaced during audit: {item}")
                        )

    all_findings = [(r["case"], f) for r in results for f in r["findings"]]
    findings = [item for item in all_findings if item not in triaged]
    accepted = [item for item in all_findings if item in triaged]
    stale = sorted(set(triaged) - set(all_findings))
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
        "corpus_roots": [str(root) for root in args.corpus],
        "baseline_accepted_by_compiler": admitted,
        "baseline_unadmitted": sum(not r.get("baseline_admitted", False) for r in results),
        "findings": len(findings),
        "triaged": len(accepted),
        "stale_triage": len(stale),
        "harness_errors": len(errors),
    }
    if args.report:
        # Summary counts alone obscure why the audit failed (e.g. an
        # accepted invalid-syntax control or a tampered package manifest).
        # Preserve structured negative evidence for independent CI checking.
        report = {
            "summary": summary,
            "results": results,
            "errors": [{"case": case, "message": message} for case, message in errors],
            "findings": [{"case": case, "reason": reason} for case, reason in findings],
            "triaged": [{"case": case, "reason": reason, "triage": triaged[(case, reason)]}
                        for case, reason in accepted],
            "stale_triage": [{"case": case, "reason": reason} for case, reason in stale],
        }
        args.report.write_text(json.dumps(report, indent=2) + "\n",
                               encoding="utf-8")
    for case, finding in findings:
        print(f"FINDING {finding}: {case}")
    for case, finding in accepted:
        print(f"TRIAGED {finding}: {case} ({triaged[(case, finding)]})")
    for case, finding in stale:
        print(f"STALE-TRIAGE {finding}: {case}")
    for case, error in errors:
        print(f"HARNESS-ERROR {case}: {error}", file=sys.stderr)
    print(json.dumps(summary))
    if errors:
        return 2
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
