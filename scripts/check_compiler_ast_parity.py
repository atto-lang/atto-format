#!/usr/bin/env python3
"""Opt-in five-backend semantic AST equivalence gate (never a parser mock).

Each independently reviewed compiler adapter must emit JSON to stdout:
  {"schema": "oreslang-semantic-ast-v1", "ast": { ... }}

The adapter must omit source locations and other nonsemantic metadata but
preserve all semantic fields. The harness never strips AST fields.
Missing compiler commands, rejected positive fixtures, malformed ASTs, or
changed trees fail closed. Offline tests validate the harness only.
"""
import argparse
from decimal import Decimal
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import stat
import tempfile

from check_compiler_parity import command_argv, run_process

BACKENDS = ("java", "llvm", "wasm", "net", "forge")
SCHEMA = "oreslang-semantic-ast-v1"
MAX_INPUT = 1024 * 1024
MAX_AST = 2 * 1024 * 1024
MAX_MANIFEST = 65536
MANIFEST_SCHEMA = "oreslang-format-fixtures-v1"
FIXTURE_NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*\.ores\Z", re.ASCII)
HEX64 = re.compile(r"[a-f0-9]{64}\Z", re.ASCII)


class InvalidAst(ValueError):
    pass


def _no_duplicates(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise InvalidAst("duplicate JSON key")
        obj[key] = value
    return obj


def _nonfinite(_value):
    raise InvalidAst("nonfinite JSON constant")


def _finite_decimal(value):
    # Do not round distinct high-precision literals to the same binary float.
    number = Decimal(value)
    if not number.is_finite() or abs(number.adjusted()) > 308:
        raise InvalidAst("nonfinite or unbounded JSON number")
    return number


def typed_ast_signature(value):
    # Python normally considers True == 1 and 1 == 1.0. Those are not the
    # same semantic AST values. Preserve JSON number/bool types and decimal
    # precision while disregarding only object key ordering.
    if isinstance(value, dict):
        return ("object", tuple(
            (key, typed_ast_signature(child))
            for key, child in sorted(value.items())
        ))
    if isinstance(value, list):
        return ("array", tuple(typed_ast_signature(child) for child in value))
    if isinstance(value, bool):
        return ("bool", value)
    if isinstance(value, int):
        return ("int", value)
    if isinstance(value, Decimal):
        return ("decimal", str(value))
    if isinstance(value, str):
        return ("string", value)
    if value is None:
        return ("null",)
    raise InvalidAst("unknown semantic AST JSON value")


def parse_ast(data):
    try:
        obj = json.loads(data.decode("utf-8"), object_pairs_hook=_no_duplicates,
                         parse_constant=_nonfinite, parse_float=_finite_decimal)
    except (ValueError, UnicodeError, RecursionError) as exc:
        raise InvalidAst("invalid AST JSON") from exc
    if (not isinstance(obj, dict) or set(obj) != {"schema", "ast"}
            or obj["schema"] != SCHEMA
            or not isinstance(obj["ast"], (dict, list)) or not obj["ast"]):
        raise InvalidAst("missing nonempty semantic AST envelope")
    return obj["ast"]


def positive_fixture(path):
    # Bounded regular-file fixture only; no symlinks, FIFOs or devices.
    if path.is_symlink():
        raise ValueError("symlink fixture refused")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    flags |= getattr(os, "O_NONBLOCK", 0)
    fd = os.open(path, flags)
    with os.fdopen(fd, "rb") as stream:
        st = os.fstat(stream.fileno())
        if not stat.S_ISREG(st.st_mode) or st.st_size > MAX_INPUT:
            raise ValueError("fixture must be a bounded regular file")
        data = stream.read(MAX_INPUT + 1)
    if len(data) > MAX_INPUT:
        raise ValueError("fixture exceeds byte limit")
    data.decode("utf-8")
    if not data.strip():
        raise ValueError("empty positive fixture")
    return data


def _read_bounded_regular(path, limit):
    # Manifest and fixture validation never follow a symlinked leaf or block
    # indefinitely on a FIFO swapped between pathname inspection and open.
    if path.is_symlink():
        raise ValueError("symlink refused")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    flags |= getattr(os, "O_NONBLOCK", 0)
    fd = os.open(path, flags)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
            raise ValueError("not a bounded regular file")
        contents = stream.read(limit + 1)
    if len(contents) > limit:
        raise ValueError("file exceeds limit")
    return contents


def load_manifest(path, positive_directory, negative_directory):
    if any(directory.is_symlink() or not directory.is_dir()
           for directory in (positive_directory, negative_directory)):
        raise ValueError("fixture directories must be regular directories")
    try:
        raw = _read_bounded_regular(path, MAX_MANIFEST)
        manifest = json.loads(raw.decode("utf-8"),
                              object_pairs_hook=_no_duplicates,
                              parse_constant=_nonfinite)
    except (OSError, UnicodeError, ValueError, RecursionError) as exc:
        raise ValueError("invalid or unreadable fixture manifest") from exc
    if (not isinstance(manifest, dict)
            or set(manifest) != {"schema", "accepted", "rejected"}
            or manifest["schema"] != MANIFEST_SCHEMA):
        raise ValueError("invalid fixture manifest schema")
    result = {}
    for field, directory in (("accepted", positive_directory),
                             ("rejected", negative_directory)):
        entries = manifest[field]
        if not isinstance(entries, list) or not 1 <= len(entries) <= 512:
            raise ValueError("required 1..512 pinned fixtures per class")
        pins = {}
        for entry in entries:
            if (not isinstance(entry, dict)
                    or set(entry) != {"name", "sha256"}
                    or not isinstance(entry["name"], str)
                    or not FIXTURE_NAME.fullmatch(entry["name"])
                    or not isinstance(entry["sha256"], str)
                    or not HEX64.fullmatch(entry["sha256"])
                    or entry["name"] in pins):
                raise ValueError("invalid or duplicate fixture manifest entry")
            pins[entry["name"]] = entry["sha256"]
        if list(pins) != sorted(pins):
            raise ValueError("fixture manifest entries must be sorted")
        actual = {item.name for item in directory.iterdir()
                  if item.name.endswith(".ores")}
        if actual != set(pins):
            raise ValueError("fixture inventory does not match pinned manifest")
        result[field] = [(directory / name, pins[name]) for name in pins]
    return result, hashlib.sha256(raw).hexdigest()


def audit_negative_fixture(path, formatter, commands, timeout, expected_sha256):
    record = {"case": path.name, "kind": "rejected", "ok": False,
              "backends": {}}
    try:
        original = positive_fixture(path)
        if hashlib.sha256(original).hexdigest() != expected_sha256:
            raise ValueError("fixture digest mismatch")
    except (OSError, ValueError, UnicodeError):
        record["error"] = "invalid_or_unpinned_fixture"
        return record
    with tempfile.TemporaryDirectory(prefix="oresfmt-negative-") as directory:
        original_file = Path(directory) / "original.ores"
        formatted_file = Path(directory) / "formatted.ores"
        original_file.write_bytes(original)
        all_original_rejected = True
        for backend in BACKENDS:
            _, status = _run(command_argv(commands[backend], original_file),
                             timeout, MAX_AST)
            record["backends"][backend] = {"original": status}
            all_original_rejected &= status == "reject"
        if not all_original_rejected:
            record["error"] = "original_not_rejected_by_all_backends"
            return record

        formatted, status = _run(formatter + ["--stdout", str(original_file)],
                                 timeout, MAX_INPUT * 2)
        record["formatter"] = status
        if status == "reject":
            # A formatter syntax rejection is safe but is NOT a format/parse
            # preservation pass. Keep the result distinguishable in receipts.
            record["outcome"] = "formatter_rejected_invalid_source"
            record["ok"] = True
            return record
        if status != "ok":
            record["error"] = "formatter_unavailable_or_failed"
            return record
        if not formatted or not formatted.strip() or len(formatted) > MAX_INPUT * 2:
            record["error"] = "invalid_formatted_output"
            return record
        try:
            formatted.decode("utf-8")
        except UnicodeError:
            record["error"] = "invalid_formatted_utf8"
            return record
        formatted_file.write_bytes(formatted)
        twice, twice_status = _run(
            formatter + ["--stdout", str(formatted_file)],
            timeout, MAX_INPUT * 2)
        record["idempotent"] = twice_status == "ok" and twice == formatted
        if not record["idempotent"]:
            record["error"] = "negative_format_not_idempotent"
            return record
        for backend in BACKENDS:
            _, after = _run(command_argv(commands[backend], formatted_file),
                            timeout, MAX_AST)
            record["backends"][backend]["formatted"] = after
        record["outcome"] = "formatted_source_remains_rejected"
        record["ok"] = all(
            entry["formatted"] == "reject"
            for entry in record["backends"].values())
        if not record["ok"]:
            record["error"] = "formatter_laundered_invalid_source"
        return record


def _run(argv, timeout, limit):
    status, exit_code, stdout, _diagnostics = run_process(
        argv, timeout, stdout_limit=limit)
    if status != "exited":
        return None, status
    if exit_code != 0:
        return None, "reject" if exit_code == 1 else "error"
    return stdout, "ok"


def audit_fixture(path, formatter, commands, timeout, expected_sha256=None):
    record = {"case": path.name, "ok": False, "backends": {}}
    try:
        original = positive_fixture(path)
        if (expected_sha256 is not None
                and hashlib.sha256(original).hexdigest() != expected_sha256):
            raise ValueError("fixture digest mismatch")
    except (OSError, ValueError, UnicodeError):
        record["error"] = "invalid_fixture"
        return record
    with tempfile.TemporaryDirectory(prefix="oresfmt-ast-") as directory:
        original_file = Path(directory) / "original.ores"
        formatted_file = Path(directory) / "formatted.ores"
        original_file.write_bytes(original)
        output, status = _run(formatter + ["--stdout", str(original_file)],
                              timeout, MAX_INPUT * 2)
        if status != "ok" or not output or len(output) > MAX_INPUT * 2:
            record["error"] = "formatter_" + status
            return record
        try:
            output.decode("utf-8")
        except UnicodeError:
            record["error"] = "formatter_invalid_utf8"
            return record
        formatted_file.write_bytes(output)
        again, second_status = _run(formatter + ["--stdout", str(formatted_file)],
                                    timeout, MAX_INPUT * 2)
        record["idempotent"] = second_status == "ok" and again == output
        if not record["idempotent"]:
            record["error"] = "not_idempotent"
            return record
        original_asts = {}
        for backend in BACKENDS:
            entry = {}
            for phase, candidate in (("original", original_file),
                                     ("formatted", formatted_file)):
                argv = command_argv(commands[backend], candidate)
                data, status = _run(argv, timeout, MAX_AST)
                if status == "ok":
                    try:
                        entry[phase] = typed_ast_signature(parse_ast(data))
                    except InvalidAst:
                        status = "invalid_ast"
                entry[phase + "_status"] = status
            entry["equivalent"] = (entry["original_status"] == "ok"
                                   and entry["formatted_status"] == "ok"
                                   and entry["original"] == entry["formatted"])
            if entry["original_status"] == "ok":
                original_asts[backend] = entry["original"]
            # Never put compiler output or private source into reports.
            record["backends"][backend] = {
                "original": entry["original_status"],
                "formatted": entry["formatted_status"],
                "equivalent": entry["equivalent"],
            }
        record["cross_backend_equivalent"] = (
            len(original_asts) == len(BACKENDS)
            and all(ast == original_asts[BACKENDS[0]]
                    for ast in original_asts.values())
        )
        record["ok"] = (record["cross_backend_equivalent"]
                        and all(x["equivalent"]
                                for x in record["backends"].values()))
        return record


def parse_commands(specs):
    commands = {}
    for spec in specs:
        backend, separator, template = spec.partition("=")
        if (not separator or backend not in BACKENDS or backend in commands
                or not template.strip()):
            raise ValueError("exactly one AST command per known backend required")
        words = shlex.split(template)
        if not words or not any("{source}" in word for word in words):
            raise ValueError("each AST command must include {source}")
        commands[backend] = template
    if set(commands) != set(BACKENDS):
        raise ValueError("AST commands required for java, llvm, wasm, net and forge")
    return commands


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--formatter", required=True,
                        help="explicit formatter command; no shell")
    parser.add_argument("--ast-command", action="append", default=[],
                        help="BACKEND='trusted compiler AST adapter {source}'")
    parser.add_argument("--cases", type=Path,
                        default=Path(__file__).resolve().parent.parent
                        / "tests" / "compiler_parity_cases")
    parser.add_argument("--negative-cases", type=Path,
                        default=Path(__file__).resolve().parent.parent
                        / "tests" / "compiler_negative_cases")
    parser.add_argument("--fixture-manifest", type=Path,
                        default=Path(__file__).resolve().parent.parent
                        / "tests" / "formatter_fixture_manifest_v1.json")
    parser.add_argument("--timeout", type=int, default=30)
    args = parser.parse_args(argv)
    if args.timeout <= 0 or args.timeout > 300:
        parser.error("timeout must be in 1..300 seconds")
    try:
        commands = parse_commands(args.ast_command)
        formatter = shlex.split(args.formatter)
        if not formatter:
            raise ValueError("empty formatter command")
    except ValueError as exc:
        parser.error(str(exc))
    try:
        corpus, manifest_digest = load_manifest(
            args.fixture_manifest, args.cases, args.negative_cases)
    except ValueError as exc:
        parser.error(str(exc))
    positive = [audit_fixture(path, formatter, commands, args.timeout, sha)
                for path, sha in corpus["accepted"]]
    negative = [audit_negative_fixture(path, formatter, commands, args.timeout, sha)
                for path, sha in corpus["rejected"]]
    ok = (all(record["ok"] for record in positive)
          and all(record["ok"] for record in negative))
    print(json.dumps({
        "ok": ok, "schema": SCHEMA, "manifest_sha256": manifest_digest,
        "required_backends": list(BACKENDS),
        "accepted_equivalent_cases": sum(x["ok"] for x in positive),
        "negative_preserved_cases": sum(x["ok"] for x in negative),
        "positive_results": positive, "negative_results": negative
    }, sort_keys=True, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
