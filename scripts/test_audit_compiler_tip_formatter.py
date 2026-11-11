"""Offline tests for the exact-head read-only formatting gate (no compiler proof)."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import audit_compiler_tip_formatter as audit


def git(root, *args):
    out = subprocess.run(
        ["git", "-C", str(root), *args],
        check=True, capture_output=True, text=True,
    )
    return out.stdout.strip()


class TipAuditTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="oresfmt-tip-test-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.formatter = self.root / "formatter"
        self.write_formatter(
            "#!/usr/bin/env python3\n"
            "import pathlib,sys\n"
            "assert sys.argv[1] == '--stdout'\n"
            "sys.stdout.buffer.write(pathlib.Path(sys.argv[2]).read_bytes())\n"
        )

    def write_formatter(self, code):
        self.formatter.write_text(code, encoding="utf-8")
        self.formatter.chmod(0o755)

    def init_repo(self, name, suffix=".atto"):
        root = self.root / name
        root.mkdir()
        git(root, "init", "-q")
        git(root, "config", "user.name", "formatter-tests")
        git(root, "config", "user.email", "formatter-tests@example.invalid")
        file = root / ("sample" + suffix)
        file.write_text("fnc answer() -> int {\n  return 42;\n}\n", encoding="utf-8")
        git(root, "add", ".")
        git(root, "commit", "-qm", "fixture")
        return root, git(root, "rev-parse", "HEAD")

    def test_inspection_requires_exact_head_and_clean_worktree(self):
        name = audit.REPOS[0]
        root, head = self.init_repo(name)
        sources = audit.inspect_repo(name, root, head)
        self.assertEqual([rel.name for rel, _ in sources], ["sample.atto"])
        with self.assertRaisesRegex(ValueError, "stale checkout"):
            audit.inspect_repo(name, root, "0" * 40)
        (root / "sample.atto").write_text("edited\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "dirty"):
            audit.inspect_repo(name, root, head)

    def test_assume_unchanged_cannot_hide_modified_compiler_source(self):
        name = audit.REPOS[0]
        root, head = self.init_repo(name)
        git(root, "update-index", "--assume-unchanged", "sample.atto")
        (root / "sample.atto").write_text("fnc forged() -> int { return 0; }\n",
                                          encoding="utf-8")
        # Git status is fooled, but immutable HEAD's blob is authoritative.
        self.assertEqual(git(root, "status", "--porcelain=v1", "--untracked-files=no"), "")
        with self.assertRaisesRegex(ValueError, "physical Git blob mismatch"):
            audit.inspect_repo(name, root, head)

    def test_formatting_stays_read_only_and_is_idempotent(self):
        root, _ = self.init_repo(audit.REPOS[0])
        source = root / "sample.atto"
        before = source.read_bytes()
        result = audit.audit_source(self.formatter, source, ".atto", 10)
        self.assertFalse(result["changed"])
        self.assertEqual(source.read_bytes(), before)
        self.write_formatter(
            "#!/usr/bin/env python3\n"
            "import pathlib,sys\n"
            "sys.stdout.buffer.write(pathlib.Path(sys.argv[2]).read_bytes() + b'X')\n"
        )
        with self.assertRaisesRegex(ValueError, "not idempotent"):
            audit.audit_source(self.formatter, source, ".atto", 10)
        self.assertEqual(source.read_bytes(), before)

    def test_full_five_backend_gate_and_json_receipt(self):
        roots = [self.init_repo(name, ".atto" if i % 2 == 0 else ".ores")
                 for i, name in enumerate(audit.REPOS)]
        args = ["--formatter", str(self.formatter)]
        for name, (root, head) in zip(audit.REPOS, roots):
            args.extend(["--repo", f"{name}={root}", "--head", f"{name}={head}"])
        report = self.root / "result.json"
        self.assertEqual(audit.main(args + ["--report", str(report)]), 0)
        receipt = json.loads(report.read_text(encoding="utf-8"))
        self.assertTrue(receipt["ok"])
        self.assertEqual(len(receipt["formatter_sha256"]), 64)
        self.assertEqual(set(receipt["repos"]), set(audit.REPOS))
        for item in receipt["repos"].values():
            self.assertEqual(item["files"], 1)
            self.assertEqual(item["unchanged"], 1)
            self.assertEqual(len(item["tree"]), 40)
        wrong = list(args)
        wrong[wrong.index(f"{audit.REPOS[4]}={roots[4][1]}")] = \
            f"{audit.REPOS[4]}={'0' * 40}"
        self.assertEqual(audit.main(wrong), 1)

    def test_late_mutation_of_earlier_source_fails_final_blob_scan(self):
        roots = [self.init_repo(name) for name in audit.REPOS]
        first_root, first_head = roots[0]
        (first_root / "later.atto").write_text(
            "fnc later() -> int {\\n  return 99;\\n}\\n", encoding="utf-8")
        git(first_root, "add", "later.atto")
        git(first_root, "commit", "-qm", "add later source")
        roots[0] = first_root, git(first_root, "rev-parse", "HEAD")
        git(first_root, "update-index", "--assume-unchanged", "sample.atto")
        self.write_formatter(
            "#!/usr/bin/env python3\\n"
            "from pathlib import Path\\n"
            "import sys\\n"
            "p=Path(sys.argv[2])\\n"
            "data=p.read_bytes()\\n"
            f"if p.name == 'later.atto' and p.parent.name == {audit.REPOS[0]!r}:\\n"
            " (p.parent/'sample.atto').write_text('tampered after preflight\\\\n')\\n"
            "sys.stdout.buffer.write(data)\\n"
        )
        args = ["--formatter", str(self.formatter)]
        for name, (root, sha) in zip(audit.REPOS, roots):
            args.extend(["--repo", f"{name}={root}", "--head", f"{name}={sha}"])
        receipt = self.root / "late-mutation.json"
        self.assertEqual(audit.main(args + ["--report", str(receipt)]), 1)
        report = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertFalse(report["ok"])
        self.assertTrue(
            any("physical Git blob mismatch" in message for message in report["failures"]),
            report["failures"]
        )

    @unittest.skipUnless(os.name == "posix", "symlink test requires POSIX")
    def test_tracked_symlink_is_rejected(self):
        name = audit.REPOS[0]
        root, _ = self.init_repo(name)
        target = root / "sample.atto"
        linked = root / "linked.atto"
        linked.symlink_to(target)
        git(root, "add", "linked.atto")
        git(root, "commit", "-qm", "linked source")
        head = git(root, "rev-parse", "HEAD")
        with self.assertRaisesRegex(ValueError, "unsafe Git tree entry|linked source component"):
            audit.inspect_repo(name, root, head)

    def test_non_utf8_source_fails_closed(self):
        root, _ = self.init_repo(audit.REPOS[0])
        source = root / "sample.atto"
        source.write_bytes(b"\xff\xfe")
        with self.assertRaisesRegex(ValueError, "non-UTF8"):
            audit.audit_source(self.formatter, source, ".atto", 10)


if __name__ == "__main__":
    unittest.main()
