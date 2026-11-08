"""Offline harness self-tests; these are NOT Java/LLVM compiler evidence."""
import json
import os
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from check_formatter_acceptance_delta import main, snapshot_corpus


class AcceptanceDeltaHarnessTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.path = Path(self.root.name)
        self.corpus = self.path / "corpus"
        self.corpus.mkdir()
        # Fake formatter: drops the word "val " (a meaning change), rejects
        # files containing "bad", and appends "!" to files containing "drift"
        # (non-idempotent).
        self.formatter = self.path / "fmt.py"
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "t=Path(sys.argv[-1]).read_text()\n"
            "if 'bad' in t: sys.exit(1)\n"
            "t=t.replace('val ', '')\n"
            "if 'drift' in t: t+='!'\n"
            "sys.stdout.write(t)\n",
            encoding="utf-8",
        )
        # Fake compiler: rejects 'write-to-val' only while 'val ' is present,
        # and rejects anything containing '!'.
        self.compiler = self.path / "cc.py"
        self.compiler.write_text(
            "import sys\nfrom pathlib import Path\n"
            "t=Path(sys.argv[-1]).read_text()\n"
            "sys.exit(1 if ('!' in t or ('write-to-val' in t and 'val ' in t)) else 0)\n",
            encoding="utf-8",
        )

    def run_main(self, *files):
        for name, text in files:
            (self.corpus / name).write_text(text, encoding="utf-8")
        report = self.path / "report.json"
        status = main([
            "--formatter", f"{sys.executable} {self.formatter}",
            "--compiler", f"cc={sys.executable} {self.compiler} {{source}}",
            "--corpus", str(self.corpus),
            "--jobs", "2",
            "--report", str(report),
        ])
        return status, report.read_text(encoding="utf-8") if report.exists() else ""

    def test_clean_corpus_passes(self):
        status, report = self.run_main(("ok.atto", "fnc main() {}\n"))
        self.assertEqual(status, 0, report)

    def test_meaning_change_is_reported_as_newly_accepted(self):
        status, report = self.run_main(("m.atto", "val x; write-to-val\n"))
        self.assertEqual(status, 1)
        self.assertIn("newly-accepted:cc", report)

    def test_formatter_rejecting_compiler_valid_source_is_reported(self):
        status, report = self.run_main(("r.atto", "bad but valid\n"))
        self.assertEqual(status, 1)
        self.assertIn("rejects-valid:cc", report)

    def test_non_idempotent_and_broken_output_are_reported(self):
        status, report = self.run_main(("d.ores", "drift\n"))
        self.assertEqual(status, 1)
        self.assertIn("not-idempotent", report)
        self.assertIn("broken:cc", report)

    def test_unrelated_extensions_are_ignored(self):
        status, report = self.run_main(("notes.txt", "bad\n"))
        self.assertEqual(status, 2, report)
        self.assertEqual(report, "")

    def test_missing_compiler_is_a_harness_error_not_a_rejection(self):
        (self.corpus / "ok.atto").write_text("fnc main() {}\n", encoding="utf-8")
        status = main([
            "--formatter", f"{sys.executable} {self.formatter}",
            "--compiler", f"cc={self.path / 'missing-compiler'} {{source}}",
            "--corpus", str(self.corpus),
        ])
        self.assertEqual(status, 2)


    def test_preserves_manifest_and_auxiliary_inputs_for_both_compiler_runs(self):
        (self.corpus / "src").mkdir()
        (self.corpus / "src" / "main.atto").write_text("fnc main() {}\n", encoding="utf-8")
        (self.corpus / ".ores.pkg.toml").write_text("[package]\nname = 'demo'\n", encoding="utf-8")
        (self.corpus / "config.json").write_text('{"enabled":true}\n', encoding="utf-8")
        self.compiler.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "root=p.parent.parent\n"
            "sys.exit(0 if (root/'.ores.pkg.toml').is_file() and "
            "(root/'config.json').is_file() else 1)\n",
            encoding="utf-8",
        )
        result, report = self.run_main()
        self.assertEqual(result, 0, report)
        self.assertIn('"baseline_accepted_by_compiler": {', report)
        self.assertIn('"baseline_unadmitted": 0', report)

    def test_all_baseline_rejections_cannot_certify_parity(self):
        self.compiler.write_text("import sys\nsys.exit(1)\n", encoding="utf-8")
        result, report = self.run_main(("rejected.atto", "fnc main() {}\n"))
        self.assertEqual(result, 2, report)
        self.assertIn('"baseline_unadmitted": 1', report)
        self.assertIn('"harness_errors": 1', report)

    def test_empty_compiler_set_is_rejected(self):
        (self.corpus / "ok.atto").write_text("fnc main() {}\n", encoding="utf-8")
        with self.assertRaises(SystemExit) as caught:
            main(["--formatter", f"{sys.executable} {self.formatter}",
                  "--corpus", str(self.corpus)])
        self.assertEqual(caught.exception.code, 2)

    def test_empty_source_inventory_does_not_pass(self):
        status, report = self.run_main(("notes.txt", "normal metadata\n"))
        self.assertEqual(status, 2)
        self.assertFalse(Path(self.path / "missing-report.json").exists())

    def test_symlinked_auxiliary_inputs_fail_closed(self):
        outside = self.path / "outside.toml"
        outside.write_text("private-data\n", encoding="utf-8")
        (self.corpus / "aux.toml").symlink_to(outside)
        status, _ = self.run_main(("ok.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)

    def test_unadmitted_negative_fixture_is_visible_but_not_silent(self):
        result, report = self.run_main(
            ("ok.atto", "fnc main() {}\n"),
            ("negative.atto", "!\n"),
        )
        self.assertEqual(result, 0, report)
        self.assertIn('"baseline_unadmitted": 1', report)


    def test_duplicate_compiler_labels_are_not_silently_overwritten(self):
        (self.corpus / "main.atto").write_text("fnc main() {}\n", encoding="utf-8")
        compiler = f"cc={sys.executable} {self.compiler} {{source}}"
        with self.assertRaises(SystemExit) as caught:
            main(["--formatter", f"{sys.executable} {self.formatter}",
                  "--compiler", compiler, "--compiler", compiler,
                  "--corpus", str(self.corpus)])
        self.assertEqual(caught.exception.code, 2)

    def test_each_compiler_requires_its_own_admitted_baseline(self):
        rejecting = self.path / "always_reject.py"
        rejecting.write_text("import sys\nsys.exit(1)\n", encoding="utf-8")
        (self.corpus / "main.atto").write_text("fnc main() {}\n", encoding="utf-8")
        report = self.path / "per-compiler.json"
        status = main([
            "--formatter", f"{sys.executable} {self.formatter}",
            "--compiler", f"working={sys.executable} {self.compiler} {{source}}",
            "--compiler", f"rejecting={sys.executable} {rejecting} {{source}}",
            "--corpus", str(self.corpus), "--report", str(report),
        ])
        self.assertEqual(status, 2)
        summary = json.loads(report.read_text())["summary"]
        self.assertEqual(summary["baseline_accepted_by_compiler"],
                         {"working": 1, "rejecting": 0})
        self.assertEqual(summary["harness_errors"], 1)

    def test_snapshot_preserves_binary_auxiliary_bytes(self):
        (self.corpus / "main.atto").write_text("fnc main() {}\n", encoding="utf-8")
        opaque = bytes([0, 255, 128, 10, 0, 42])
        (self.corpus / "resource.bin").write_bytes(opaque)
        output = self.path / "snapshot"
        self.assertEqual(snapshot_corpus(self.corpus, output), 2)
        self.assertEqual((output / "resource.bin").read_bytes(), opaque)
        self.assertEqual((output / "main.atto").read_bytes(),
                         (self.corpus / "main.atto").read_bytes())

    def test_snapshot_refuses_file_count_budget_overrun(self):
        (self.corpus / "resource.bin").write_bytes(b"x")
        with mock.patch("check_formatter_acceptance_delta.MAX_SNAPSHOT_FILES", 1):
            status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)
        self.assertEqual(report, "")

    def test_snapshot_refuses_byte_budget_overrun(self):
        with mock.patch("check_formatter_acceptance_delta.MAX_SNAPSHOT_BYTES", 2):
            status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)
        self.assertEqual(report, "")

    def test_snapshot_refuses_oversized_auxiliary_file(self):
        (self.corpus / "resource.bin").write_bytes(b"123456")
        with mock.patch("check_formatter_acceptance_delta.MAX_SOURCE_BYTES", 5):
            status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)
        self.assertEqual(report, "")

    @unittest.skipUnless(os.name == "posix", "requires POSIX symlinks")
    def test_snapshot_refuses_symlinked_directory(self):
        outside = self.path / "out"
        outside.mkdir()
        (self.corpus / "linked").symlink_to(outside, target_is_directory=True)
        status, _ = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)

    @unittest.skipUnless(os.name == "posix", "requires POSIX symlinks")
    def test_snapshot_refuses_symlinked_root(self):
        alias = self.path / "corpus_alias"
        alias.symlink_to(self.corpus, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            snapshot_corpus(alias, self.path / "snapshot")

    @unittest.skipUnless(hasattr(os, "mkfifo"), "requires POSIX FIFO")
    def test_snapshot_refuses_special_files_without_opening_them(self):
        os.mkfifo(self.corpus / "named_pipe")
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)
        self.assertEqual(report, "")

    def test_formatter_only_mutates_isolated_copy(self):
        text = "val x; write-to-val\n"
        status, report = self.run_main(("m.atto", text))
        self.assertEqual(status, 1, report)
        self.assertEqual((self.corpus / "m.atto").read_text(encoding="utf-8"), text)
        self.assertIn("newly-accepted:cc", report)

    def test_empty_inventory_fails_without_claiming_a_report(self):
        status, report = self.run_main(("README.md", "this is not source\n"))
        self.assertEqual(status, 2)
        self.assertEqual(report, "")


if __name__ == "__main__":
    unittest.main()
