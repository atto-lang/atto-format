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

    def test_git_control_environment_cannot_redirect_source_identity(self):
        from unittest import mock
        name = audit.REPOS[0]
        root, head = self.init_repo(name)
        other, _ = self.init_repo("foreign-checkout")
        # Git would normally let GIT_DIR / GIT_WORK_TREE override -C.
        # The audit must still inventory the intended exact worktree.
        with mock.patch.dict(os.environ, {
            "GIT_DIR": str(other / ".git"),
            "GIT_WORK_TREE": str(other),
        }):
            files = audit.inspect_repo(name, root, head)
        self.assertEqual([rel.name for rel, _ in files], ["sample.atto"])

    def test_git_replace_cannot_substitute_untrusted_head_tree(self):
        name = audit.REPOS[0]
        root, good_head = self.init_repo(name)
        source = root / "sample.atto"
        forged_source = "fnc forged() -> int {\n  return 0;\n}\n"
        source.write_text(forged_source, encoding="utf-8")
        git(root, "add", "sample.atto")
        git(root, "commit", "-qm", "evil alternate tree")
        alternate_head = git(root, "rev-parse", "HEAD")
        git(root, "checkout", "-q", "--detach", good_head)
        git(root, "replace", good_head, alternate_head)
        # Stage the forged tree without advancing HEAD. With replacement
        # objects enabled, both 'status' and 'ls-tree HEAD' now look clean.
        git(root, "read-tree", alternate_head)
        source.write_text(forged_source, encoding="utf-8")
        self.assertEqual(git(root, "status", "--porcelain=v1", "--untracked-files=no"), "")
        # The auditor disables replacement objects, revealing the staged
        # tree mismatch even though the reported HEAD SHA is unchanged.
        with self.assertRaisesRegex(ValueError, "tracked worktree is dirty|physical Git blob mismatch"):
            audit.inspect_repo(name, root, good_head)

    def test_nested_directory_cannot_impersonate_repository_root(self):
        name = audit.REPOS[0]
        root, head = self.init_repo(name)
        nested = root / "src"
        nested.mkdir()
        with self.assertRaisesRegex(ValueError, "not an exact Git repository root"):
            audit.inspect_repo(name, nested, head)

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

    def test_untracked_and_ignored_inputs_fail_closed(self):
        name = audit.REPOS[0]
        root, head = self.init_repo(name)
        added = root / "untracked.toml"
        added.write_text("untracked config", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "untracked worktree files"):
            audit.inspect_repo(name, root, head)
        added.unlink()
        (root / ".gitignore").write_text("hidden-config.toml\n", encoding="utf-8")
        git(root, "add", ".gitignore")
        git(root, "commit", "-qm", "ignore sensitive config")
        head = git(root, "rev-parse", "HEAD")
        hidden = root / "hidden-config.toml"
        hidden.write_text("hidden config", encoding="utf-8")
        self.assertEqual(git(root, "status", "--porcelain=v1", "--untracked-files=no"), "")
        with self.assertRaisesRegex(ValueError, "ignored worktree files"):
            audit.inspect_repo(name, root, head)

    def test_non_source_manifest_assume_unchanged_is_verified(self):
        name = audit.REPOS[0]
        root, head = self.init_repo(name)
        manifest = root / ".ores.pkg.toml"
        manifest.write_text("[package]\nname = 'demo'\n", encoding="utf-8")
        git(root, "add", ".ores.pkg.toml")
        git(root, "commit", "-qm", "tracked manifest")
        head = git(root, "rev-parse", "HEAD")
        git(root, "update-index", "--assume-unchanged", ".ores.pkg.toml")
        manifest.write_text("[package]\nname = 'forged'\n", encoding="utf-8")
        self.assertEqual(git(root, "status", "--porcelain=v1", "--untracked-files=no"), "")
        with self.assertRaisesRegex(ValueError, "physical Git blob mismatch"):
            audit.inspect_repo(name, root, head)

    def test_manifest_mutated_during_formatter_run_fails_final_scan(self):
        roots = [self.init_repo(name) for name in audit.REPOS]
        root, _ = roots[0]
        manifest = root / ".ores.pkg.toml"
        manifest.write_text("[package]\nname = 'demo'\n", encoding="utf-8")
        git(root, "add", ".ores.pkg.toml")
        git(root, "commit", "-qm", "tracked manifest")
        roots[0] = root, git(root, "rev-parse", "HEAD")
        git(root, "update-index", "--assume-unchanged", ".ores.pkg.toml")
        self.write_formatter(
            "#!/usr/bin/env python3\n"
            "from pathlib import Path\n"
            "import sys\n"
            "p = Path(sys.argv[2])\n"
            "if p.parent.name == 'oreslang-source.java':\n"
            " (p.parent/'.ores.pkg.toml').write_text('[package]\\nname = \"forged\"\\n')\n"
            "sys.stdout.buffer.write(p.read_bytes())\n"
        )
        args = ["--formatter", str(self.formatter)]
        for name, (where, sha) in zip(audit.REPOS, roots):
            args.extend(["--repo", f"{name}={where}", "--head", f"{name}={sha}"])
        receipt = self.root / "manifest-attack.json"
        self.assertEqual(audit.main(args + ["--report", str(receipt)]), 1)
        row = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertFalse(row["ok"])
        self.assertTrue(any(".ores.pkg.toml" in message and "physical Git blob mismatch" in message
                            for message in row["failures"]), row["failures"])

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
        self.assertEqual(
            sum(item["tracked_files_verified"] for item in receipt["repos"].values()), 5
        )
        self.assertGreater(
            sum(item["tracked_bytes_verified"] for item in receipt["repos"].values()), 0
        )
        for item in receipt["repos"].values():
            self.assertEqual(item["files"], 1)
            self.assertEqual(item["unchanged"], 1)
            self.assertEqual(len(item["tree"]), 40)
        wrong = list(args)
        wrong[wrong.index(f"{audit.REPOS[4]}={roots[4][1]}")] = \
            f"{audit.REPOS[4]}={'0' * 40}"
        self.assertEqual(audit.main(wrong), 1)

    def test_pinned_inventory_rejects_mismatched_counts_and_duplicate_keys(self):
        roots = [self.init_repo(name, ".atto" if i % 2 == 0 else ".ores")
                 for i, name in enumerate(audit.REPOS)]
        entries = {}
        args = ["--formatter", str(self.formatter)]
        for name, (root, sha) in zip(audit.REPOS, roots):
            args.extend(["--repo", f"{name}={root}", "--head", f"{name}={sha}"])
            source = next(root.glob("*.atto" if name in (audit.REPOS[0],
                                                         audit.REPOS[2],
                                                         audit.REPOS[4])
                                    else "*.ores"))
            entries[name] = {
                "head": sha, "tree": git(root, "rev-parse", "HEAD^{tree}"),
                "tracked_files": 1, "tracked_bytes": source.stat().st_size,
                "source_files": 1,
                "atto_files": int(source.suffix == ".atto"),
                "ores_files": int(source.suffix == ".ores"),
            }
        manifest = self.root / "inventory.json"
        manifest.write_text(json.dumps({
            "schema": "oresfmt-five-backend-tip-inventory-v1",
            "source": "immutable-github-git-tree-metadata-only",
            "repos": entries,
        }), encoding="utf-8")
        report = self.root / "inventory-receipt.json"
        self.assertEqual(audit.main(args + [
            "--inventory", str(manifest), "--report", str(report)
        ]), 0)
        row = json.loads(report.read_text(encoding="utf-8"))
        self.assertEqual(len(row["inventory_sha256"]), 64)
        self.assertEqual(sum(x["atto_files"] for x in row["repos"].values()), 3)

        entries[audit.REPOS[0]]["tracked_bytes"] += 1
        manifest.write_text(json.dumps({
            "schema": "oresfmt-five-backend-tip-inventory-v1",
            "source": "immutable-github-git-tree-metadata-only",
            "repos": entries,
        }), encoding="utf-8")
        self.assertEqual(audit.main(args + [
            "--inventory", str(manifest), "--report", str(report)
        ]), 1)
        self.assertTrue(any("inventory tracked_bytes mismatch" in x
                            for x in json.loads(report.read_text())["failures"]))
        manifest.write_text('{"schema":"x","schema":"y"}', encoding="utf-8")
        with self.assertRaises(SystemExit) as error:
            audit.main(args + ["--inventory", str(manifest)])
        self.assertEqual(error.exception.code, 2)

    def test_late_mutation_of_earlier_source_fails_final_blob_scan(self):
        roots = [self.init_repo(name) for name in audit.REPOS]
        first_root, first_head = roots[0]
        (first_root / "later.atto").write_text(
            "fnc later() -> int {\n  return 99;\n}\n", encoding="utf-8")
        git(first_root, "add", "later.atto")
        git(first_root, "commit", "-qm", "add later source")
        roots[0] = first_root, git(first_root, "rev-parse", "HEAD")
        git(first_root, "update-index", "--assume-unchanged", "sample.atto")
        self.write_formatter(
            "#!/usr/bin/env python3\n"
            "from pathlib import Path\n"
            "import sys\n"
            "p=Path(sys.argv[2])\n"
            "data=p.read_bytes()\n"
            f"if p.name == 'later.atto' and p.parent.name == {audit.REPOS[0]!r}:\n"
            " (p.parent/'sample.atto').write_text('tampered after preflight\\\n')\n"
            "sys.stdout.buffer.write(data)\n"
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

    def test_formatter_exit_one_is_rejection_but_exit_two_is_tool_error(self):
        root, _ = self.init_repo(audit.REPOS[0])
        source = root / "sample.atto"
        self.write_formatter("#!/usr/bin/env python3\nimport sys\nsys.exit(1)\n")
        with self.assertRaises(audit.FormatterRejected):
            audit.audit_source(self.formatter, source, ".atto", 10)
        self.write_formatter("#!/usr/bin/env python3\nimport sys\nsys.exit(2)\n")
        with self.assertRaisesRegex(ValueError, "formatter tool failure"):
            audit.audit_source(self.formatter, source, ".atto", 10)

    def test_formatter_rejection_receipt_cannot_pass_without_compiler_evidence(self):
        roots = [self.init_repo(name) for name in audit.REPOS]
        self.write_formatter(
            "#!/usr/bin/env python3\n"
            "import sys\n"
            "sys.exit(1 if 'oreslang-source.java' in sys.argv[2] else 0)\n"
        )
        args = ["--formatter", str(self.formatter)]
        for name, (root, head) in zip(audit.REPOS, roots):
            args.extend(["--repo", f"{name}={root}", "--head", f"{name}={head}"])
        report = self.root / "reject-report.json"
        self.assertEqual(audit.main(args + ["--report", str(report)]), 1)
        data = json.loads(report.read_text(encoding="utf-8"))
        self.assertFalse(data["ok"])
        self.assertEqual(data["repos"][audit.REPOS[0]]["rejected"], 1)
        self.assertEqual(data["repos"][audit.REPOS[0]]["format_errors"], 0)
        self.assertEqual(len(data["rejections"]), 1)
        self.assertEqual(data["rejections"][0]["status"], "requires-compiler-baseline")

    @unittest.skipUnless(os.name == "posix", "secure nofollow read requires POSIX")
    def test_formatter_cannot_hide_leaf_symlink_in_post_format_read(self):
        root, _ = self.init_repo(audit.REPOS[0])
        target = self.root / "out-of-repository.atto"
        target.write_text("outside\n", encoding="utf-8")
        self.write_formatter(
            "#!/usr/bin/env python3\n"
            "from pathlib import Path\n"
            "import sys\n"
            "p = Path(sys.argv[2])\n"
            "data = p.read_bytes()\n"
            "if p.name == 'sample.atto':\n"
            " p.unlink()\n"
            f" p.symlink_to({str(target)!r})\n"
            "sys.stdout.buffer.write(data)\n"
        )
        with self.assertRaises(OSError):
            audit.audit_source(self.formatter, root / "sample.atto", ".atto", 10)

    def test_non_utf8_source_fails_closed(self):
        root, _ = self.init_repo(audit.REPOS[0])
        source = root / "sample.atto"
        source.write_bytes(b"\xff\xfe")
        with self.assertRaisesRegex(ValueError, "non-UTF8"):
            audit.audit_source(self.formatter, source, ".atto", 10)


if __name__ == "__main__":
    unittest.main()
