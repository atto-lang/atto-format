"""Source-free harness self-tests, NOT real Oreslang compiler evidence."""
from pathlib import Path
import hashlib
import json
import sys
import tempfile
import unittest

from check_compiler_ast_parity import (BACKENDS, InvalidAst, audit_fixture,
                                       audit_negative_fixture, load_manifest,
                                       load_golden_manifest, main, parse_ast,
                                       parse_commands, semantic_ast_sha256,
                                       typed_ast_signature)


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

    def pinned_manifest(self, corrupted=False):
        root = Path(self.temp.name) / "pinned"
        accepted = root / "accepted"
        rejected = root / "rejected"
        accepted.mkdir(parents=True, exist_ok=True)
        rejected.mkdir()
        (accepted / "good.ores").write_bytes(self.case.read_bytes())
        (rejected / "bad.ores").write_text("invalid syntax\n", encoding="utf-8")

        def pin(directory, name):
            contents = (directory / name).read_bytes()
            digest = hashlib.sha256(contents).hexdigest()
            if corrupted:
                digest = "0" * 64
            return {"name": name, "sha256": digest}

        manifest = root / "manifest.json"
        manifest.write_text(json.dumps({
            "schema": "oreslang-format-fixtures-v1",
            "accepted": [pin(accepted, "good.ores")],
            "rejected": [pin(rejected, "bad.ores")],
        }), encoding="utf-8")
        return accepted, rejected, manifest

    def synthetic_goldens(self, accepted):
        # This is a toy AST receipt: use only to test the validator.
        path = accepted.parent / "golden.json"
        digest = semantic_ast_sha256({"tokens": ["hello", "world"]})
        path.write_text(json.dumps({
            "schema": "oreslang-semantic-ast-golden-v1",
            "cases": [{"name": "good.ores", "ast_sha256": digest}],
        }), encoding="utf-8")
        return path

    def test_missing_wrong_or_unpinned_ast_goldens_fail(self):
        accepted, rejected, manifest = self.pinned_manifest()
        golden = self.synthetic_goldens(accepted)
        pins, receipt = load_golden_manifest(golden, ["good.ores"])
        self.assertEqual(len(receipt), 64)
        self.assertEqual(set(pins), {"good.ores"})
        self.assertEqual(pins["good.ores"],
                         semantic_ast_sha256({"tokens": ["hello", "world"]}))
        with self.assertRaises(ValueError):
            load_golden_manifest(accepted / "missing.json", ["good.ores"])
        with self.assertRaises(ValueError):
            load_golden_manifest(golden, ["different.ores"])
        with self.assertRaises(ValueError):
            load_golden_manifest(golden, ["good.ores", "second.ores"])
        bad_pin = "0" * 64
        report = audit_fixture(
            accepted / "good.ores", self.format_cmd, self.commands, 5,
            expected_ast_sha256=bad_pin)
        self.assertFalse(report["ok"])
        self.assertFalse(report["golden_verified"])

    def negative_compiler(self):
        self.compiler.write_text(
            "import sys, json\nfrom pathlib import Path\n"
            "value=Path(sys.argv[-1]).read_text()\n"
            "if 'invalid' in value: sys.exit(1)\n"
            "print(json.dumps({'schema': 'oreslang-semantic-ast-v1', "
            "'ast': {'tokens': value.split()}}))\n", encoding="utf-8")

    def test_pinned_manifest_and_negative_corpus(self):
        self.negative_compiler()
        accepted, rejected, manifest = self.pinned_manifest()
        cases, digest = load_manifest(manifest, accepted, rejected)
        self.assertEqual(len(cases["accepted"]), 1)
        self.assertEqual(len(cases["rejected"]), 1)
        self.assertEqual(len(digest), 64)
        case, pin = cases["rejected"][0]
        report = audit_negative_fixture(
            case, self.format_cmd, self.commands, 5, pin)
        self.assertTrue(report["ok"], report)
        self.assertEqual(report["outcome"], "formatted_source_remains_rejected")

    def test_fixture_digest_tampering_fails_before_execution(self):
        accepted, rejected, manifest = self.pinned_manifest(corrupted=True)
        with self.assertRaisesRegex(ValueError, "reviewed SHA-256"):
            load_manifest(manifest, accepted, rejected)
        # Independently recheck the actual bytes used even if a fixture
        # changes after initial manifest validation.
        self.assertEqual(audit_fixture(
            accepted / "good.ores", self.format_cmd, self.commands,
            5, "0" * 64)["error"], "invalid_fixture")
        self.assertEqual(audit_negative_fixture(
            rejected / "bad.ores", self.format_cmd, self.commands,
            5, "0" * 64)["error"], "invalid_or_unpinned_fixture")

    def test_manifest_inventory_and_traversal_denied(self):
        accepted, rejected, manifest = self.pinned_manifest()
        (rejected / "surprise.ores").write_text("invalid", encoding="utf-8")
        with self.assertRaises(ValueError):
            load_manifest(manifest, accepted, rejected)
        (rejected / "surprise.ores").unlink()
        spec = json.loads(manifest.read_text())
        spec["accepted"][0]["name"] = "../good.ores"
        manifest.write_text(json.dumps(spec))
        with self.assertRaises(ValueError):
            load_manifest(manifest, accepted, rejected)
        spec["accepted"][0]["name"] = "good.ores"
        spec["rejected"].append(spec["rejected"][0])
        manifest.write_text(json.dumps(spec))
        with self.assertRaises(ValueError):
            load_manifest(manifest, accepted, rejected)

    def test_manifest_duplicate_key_and_missing_negatives_denied(self):
        accepted, rejected, manifest = self.pinned_manifest()
        manifest.write_text('{"schema":"oreslang-format-fixtures-v1",'
                            '"schema":"oreslang-format-fixtures-v1",'
                            '"accepted":[],"rejected":[]}')
        with self.assertRaises(ValueError):
            load_manifest(manifest, accepted, rejected)
        spec = {"schema": "oreslang-format-fixtures-v1",
                "accepted": [{"name": "good.ores", "sha256": "0" * 64}],
                "rejected": []}
        manifest.write_text(json.dumps(spec))
        with self.assertRaises(ValueError):
            load_manifest(manifest, accepted, rejected)

    def test_negative_source_laundered_into_valid_is_blocked(self):
        self.negative_compiler()
        case = Path(self.temp.name) / "bad.ores"
        case.write_text("invalid  syntax\n", encoding="utf-8")
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "value=Path(sys.argv[-1]).read_text()\n"
            "print(value.replace('invalid', 'hello').replace('  ', ' '), end='')\n")
        report = audit_negative_fixture(
            case, self.format_cmd, self.commands, 5,
            hashlib.sha256(case.read_bytes()).hexdigest())
        self.assertFalse(report["ok"], report)
        self.assertEqual(report["error"], "formatter_laundered_invalid_source")

    def test_formatter_syntax_rejection_is_distinct_safe_outcome(self):
        self.negative_compiler()
        case = Path(self.temp.name) / "bad.ores"
        case.write_text("invalid syntax\n", encoding="utf-8")
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "value=Path(sys.argv[-1]).read_text()\n"
            "sys.exit(1 if 'invalid' in value else 0)\n")
        report = audit_negative_fixture(
            case, self.format_cmd, self.commands, 5,
            hashlib.sha256(case.read_bytes()).hexdigest())
        self.assertTrue(report["ok"], report)
        self.assertEqual(report["outcome"],
                         "formatter_rejected_invalid_source")

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

    def test_unrepresentable_json_exponent_is_invalid_ast(self):
        invalid = (b'{"schema":"oreslang-semantic-ast-v1",'
                   b'"ast":{"literal":1e' + b'9' * 22 + b'}}')
        with self.assertRaises(InvalidAst):
            parse_ast(invalid)

    def test_semantic_ast_types_and_precision_are_not_collapsed(self):
        # Python's default equality would wrongly equate bool/int/float.
        def signature(atom):
            payload = ('{"schema":"oreslang-semantic-ast-v1",'
                       '"ast":{"literal":' + atom + '}}').encode()
            return typed_ast_signature(parse_ast(payload))
        self.assertNotEqual(signature("true"), signature("1"))
        self.assertNotEqual(signature("1"), signature("1.0"))
        self.assertNotEqual(signature("0.123456789012345678901"),
                            signature("0.123456789012345678902"))
        self.assertEqual(signature('{"b":2,"a":1}'),
                         signature('{"a":1,"b":2}'))
        self.assertNotEqual(signature("[1,2]"), signature("[2,1]"))

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
        self.negative_compiler()
        accepted, rejected, manifest = self.pinned_manifest()
        commands = sum((["--ast-command", f"{backend}={self.ast_cmd}"]
                        for backend in BACKENDS), [])
        golden = self.synthetic_goldens(accepted)
        args = (["--formatter", f'"{sys.executable}" "{self.formatter}"',
                 "--cases", str(accepted),
                 "--negative-cases", str(rejected),
                 "--fixture-manifest", str(manifest),
                 "--golden-manifest", str(golden)] + commands)
        self.assertEqual(main(args), 0)
        with self.assertRaises(SystemExit) as context:
            main(args[:-2])
        self.assertEqual(context.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
