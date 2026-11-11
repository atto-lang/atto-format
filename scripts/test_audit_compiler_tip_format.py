"""Offline self-tests; simulated formatter is NOT compiler parity evidence."""
import sys
import tempfile
import unittest
from pathlib import Path
import subprocess

sys.path.insert(0, str(Path(__file__).resolve().parent))
from audit_compiler_tip_format import audit_repo, repo_argument


class CompilerTipAuditTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="oresfmt-tip-tests-")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name).resolve()
        self.repo = self.root / "compiler"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "formatter-self-test")
        self.git("config", "user.email", "formatter@example.invalid")
        self.formatter = self.root / "formatter.py"
        self.formatter.write_text(
            'import pathlib, sys\n'
            'source = pathlib.Path(sys.argv[-1]).read_text()\n'
            'sys.stdout.write(source.replace("return   42;", "return 42;"))\n',
            encoding="utf-8",
        )

    def git(self, *args):
        return subprocess.check_output(
            ["git", "-C", str(self.repo), *args], stderr=subprocess.PIPE
        ).decode().strip()

    def commit(self, relative="src/main.atto", source="return   42;\n"):
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")
        self.git("add", ".")
        self.git("commit", "-qm", "source")
        return self.git("rev-parse", "HEAD")

    def audit(self, expected, clean=False):
        return audit_repo(
            "java", self.repo, expected, [sys.executable, str(self.formatter)],
            timeout=5, max_bytes=1024, max_files=10, require_clean=clean,
        )

    def test_pinned_head_roundtrips_and_tracks_changes(self):
        head = self.commit()
        result = self.audit(head)
        self.assertTrue(result["ok"], result)
        self.assertEqual(result["actual_head"], head)
        self.assertEqual(result["scanned"], 1)
        self.assertEqual(result["changed"], 1)
        self.assertRegex(result["source_manifest_sha256"], r"^[0-9a-f]{64}$")
        self.assertEqual((self.repo / "src/main.atto").read_text(), "return   42;\n")

    def test_require_clean_detects_uncanonical_sources_without_writing(self):
        head = self.commit()
        result = self.audit(head, clean=True)
        self.assertFalse(result["ok"])
        self.assertIn("differs from canonical", result["failures"][0]["error"])

    def test_stale_head_and_dirty_checkout_fail_before_formatting(self):
        head = self.commit()
        result = self.audit("0" * 40)
        self.assertFalse(result["ok"])
        self.assertEqual(result["scanned"], 0)
        self.assertIn("HEAD does not match", result["failures"][0]["error"])
        (self.repo / "src/main.atto").write_text("return 12;\n")
        result = self.audit(head)
        self.assertFalse(result["ok"])
        self.assertEqual(result["scanned"], 0)
        self.assertIn("uncommitted tracked changes", result["failures"][0]["error"])

    def test_non_idempotent_formatter_is_rejected(self):
        head = self.commit()
        self.formatter.write_text(
            'import pathlib, sys\n'
            'sys.stdout.write(pathlib.Path(sys.argv[-1]).read_text() + "X")\n',
            encoding="utf-8",
        )
        result = self.audit(head)
        self.assertFalse(result["ok"])
        self.assertIn("not idempotent", result["failures"][0]["error"])

    def test_symlinked_tracked_sources_fail_closed(self):
        head = self.commit()
        (self.repo / "linked.atto").symlink_to("src/main.atto")
        self.git("add", "linked.atto")
        self.git("commit", "-qm", "symlink fixture")
        result = self.audit(self.git("rev-parse", "HEAD"))
        self.assertFalse(result["ok"])
        self.assertEqual(result["scanned"], 2)
        self.assertTrue(
            any("symlink" in item["error"] for item in result["failures"])
        )

    def test_empty_and_oversized_corpora_are_not_successes(self):
        self.git("commit", "--allow-empty", "-qm", "empty")
        result = self.audit(self.git("rev-parse", "HEAD"))
        self.assertFalse(result["ok"])
        self.assertIn("no tracked", result["failures"][0]["error"])
        head = self.commit(source="a" * 2048)
        result = self.audit(head)
        self.assertFalse(result["ok"])
        self.assertIn("bounded regular file", result["failures"][0]["error"])

    def test_argument_rejects_unknown_backend_and_partial_sha(self):
        with self.assertRaises(Exception):
            repo_argument("python=/tmp/source@" + "a" * 40)
        with self.assertRaises(Exception):
            repo_argument("java=/tmp/source@abc")


if __name__ == "__main__":
    unittest.main()
