"""Source-free harness self-tests, NOT real Oreslang compiler evidence."""
from pathlib import Path
import sys
import tempfile
import unittest

from check_compiler_ast_parity import (BACKENDS, InvalidAst, audit_fixture,
                                       main, parse_ast, parse_commands)


class AstParityHarnessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.case = root / "good.ores"
        self.case.write_text("hello  world\n", encoding="utf-8")
        self.formatter = root / "format.py"
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "print(Path(sys.argv[-1]).read_text().replace('  ', ' '), end='')\n",
            encoding="utf-8",
        )
        self.compiler = root / "compiler.py"
        self.compiler.write_text(
            "import sys, json\nfrom pathlib import Path\n"
            "value=Path(sys.argv[-1]).read_text().split()\n"
            "print(json.dumps({'schema': 'oreslang-semantic-ast-v1', "
            "'ast': {'tokens': value}}))\n",
            encoding="utf-8",
        )
        self.format_cmd = [sys.executable, str(self.formatter)]
        self.ast_cmd = f'"{sys.executable}" "{self.compiler}" {{source}}'
        self.commands = {backend: self.ast_cmd for backend in BACKENDS}

    def test_all_five_equivalent_after_format(self):
        record = audit_fixture(self.case, self.format_cmd, self.commands, 5)
        self.assertTrue(record["ok"], record)
        self.assertTrue(record["cross_backend_equivalent"])
        self.assertEqual(set(record["backends"]), set(BACKENDS))

    def test_ast_changed_across_format_fails(self):
        self.compiler.write_text(
            "import sys, json\nfrom pathlib import Path\n"
            "print(json.dumps({'schema': 'oreslang-semantic-ast-v1', "
            "'ast': {'bytes': Path(sys.argv[-1]).read_text()}}))\n",
            encoding="utf-8",
        )
        result = audit_fixture(self.case, self.format_cmd, self.commands, 5)
        self.assertFalse(result["ok"])
        self.assertFalse(result["backends"]["java"]["equivalent"])

    def test_cross_backend_divergence_fails(self):
        other = Path(self.temp.name) / "diverge.py"
        other.write_text(
            "import json\n"
            "print(json.dumps({'schema': 'oreslang-semantic-ast-v1', "
            "'ast': {'tokens': ['different']}}))\n",
            encoding="utf-8",
        )
        self.commands["forge"] = f'"{sys.executable}" "{other}" {{source}}'
        result = audit_fixture(self.case, self.format_cmd, self.commands, 5)
        self.assertFalse(result["ok"])
        self.assertTrue(result["backends"]["forge"]["equivalent"])
        self.assertFalse(result["cross_backend_equivalent"])

    def test_missing_compiler_cannot_pass(self):
        self.commands["net"] = "/definitely/missing/compiler {source}"
        record = audit_fixture(self.case, self.format_cmd, self.commands, 5)
        self.assertFalse(record["ok"])
        self.assertEqual(record["backends"]["net"]["original"], "unavailable")

    def test_schema_missing_duplicate_nonfinite_empty_and_scalar_denied(self):
        bad = [
            b'{}', b'[]', b'null', b'"hello"',
            b'{"schema":"oreslang-semantic-ast-v1","ast":{}}',
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1},"ast":{"x":2}}',
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":NaN}}',
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1e10000}}',
            b'not json', b'\xff',
        ]
        for candidate in bad:
            with self.subTest(candidate=candidate), self.assertRaises(InvalidAst):
                parse_ast(candidate)
        self.assertEqual(
            parse_ast(b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1}}'),
            {"x": 1})

    def test_parse_commands_requires_exactly_five(self):
        specs = [f"{backend}={self.ast_cmd}" for backend in BACKENDS]
        self.assertEqual(parse_commands(specs), self.commands)
        for invalid in (specs[:-1], specs + [specs[0]],
                        specs[:-1] + ["unknown=exec {source}"]):
            with self.assertRaises(ValueError):
                parse_commands(invalid)

    def test_symlink_and_empty_fixture_fail(self):
        target = Path(self.temp.name) / "target"
        target.write_text("hello", encoding="utf-8")
        link = Path(self.temp.name) / "linked.ores"
        link.symlink_to(target)
        self.assertEqual(
            audit_fixture(link, self.format_cmd, self.commands, 5)["error"],
            "invalid_fixture")
        self.case.write_text("  \n", encoding="utf-8")
        self.assertEqual(
            audit_fixture(self.case, self.format_cmd, self.commands, 5)["error"],
            "invalid_fixture")

    def test_cli_requires_five_commands_and_positive_cases(self):
        root = Path(self.temp.name) / "cases"
        root.mkdir()
        (root / "good.ores").write_bytes(self.case.read_bytes())
        commands = sum((["--ast-command", f"{backend}={self.ast_cmd}"]
                        for backend in BACKENDS), [])
        args = (["--formatter", f'"{sys.executable}" "{self.formatter}"',
                 "--cases", str(root)] + commands)
        self.assertEqual(main(args), 0)
        with self.assertRaises(SystemExit) as context:
            main(args[:-2])
        self.assertEqual(context.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
