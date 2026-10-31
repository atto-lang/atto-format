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
import json
import math
import os
from pathlib import Path
import shlex
import stat
import tempfile

from check_compiler_parity import command_argv, run_process

BACKENDS = ("java", "llvm", "wasm", "net", "forge")
SCHEMA = "oreslang-semantic-ast-v1"
MAX_INPUT = 1024 * 1024
MAX_AST = 2 * 1024 * 1024


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


def _finite_float(value):
    parsed = float(value)
    if not math.isfinite(parsed):
        raise InvalidAst("nonfinite JSON number")
    return parsed


def parse_ast(data):
    try:
        obj = json.loads(data.decode("utf-8"), object_pairs_hook=_no_duplicates,
                         parse_constant=_nonfinite, parse_float=_finite_float)
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


def _run(argv, timeout, limit):
    status, exit_code, stdout, _diagnostics = run_process(
        argv, timeout, stdout_limit=limit)
    if status != "exited":
        return None, status
    if exit_code != 0:
        return None, "reject" if exit_code == 1 else "error"
    return stdout, "ok"


def audit_fixture(path, formatter, commands, timeout):
    record = {"case": path.name, "ok": False, "backends": {}}
    try:
        original = positive_fixture(path)
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
                        entry[phase] = parse_ast(data)
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
    if not args.cases.is_dir() or args.cases.is_symlink():
        parser.error("cases must be a real directory")
    cases = sorted(args.cases.glob("*.ores"))
    if not cases or len(cases) > 512:
        parser.error("expected 1..512 positive .ores fixtures")
    results = [audit_fixture(path, formatter, commands, args.timeout)
               for path in cases]
    ok = bool(results) and all(record["ok"] for record in results)
    print(json.dumps({"ok": ok, "schema": SCHEMA,
                      "required_backends": list(BACKENDS),
                      "accepted_equivalent_cases": sum(x["ok"] for x in results),
                      "results": results}, sort_keys=True, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
