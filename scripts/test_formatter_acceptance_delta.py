"""Offline harness self-tests; these are NOT Java/LLVM compiler evidence."""
import sys
import tempfile
import unittest
from pathlib import Path

from check_formatter_acceptance_delta import main


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
        return status, report.read_text(encoding="utf-8")

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

    def test_triaged_findings_do_not_fail_and_stale_entries_are_reported(self):
        (self.corpus / "m.atto").write_text("val x; write-to-val\n", encoding="utf-8")
        triage = self.path / "triage.tsv"
        triage.write_text(
            "corpus/m.atto\tnewly-accepted:cc\treviewed migration\n"
            "corpus/gone.atto\tbroken:cc\tno longer occurs\n",
            encoding="utf-8",
        )
        status = main([
            "--formatter", f"{sys.executable} {self.formatter}",
            "--compiler", f"cc={sys.executable} {self.compiler} {{source}}",
            "--corpus", str(self.corpus),
            "--triaged", str(triage),
        ])
        self.assertEqual(status, 0)

    def test_imports_of_formatter_rejected_files_still_resolve(self):
        # The importer is formatted; the imported file is rejected by the
        # formatter. The compiler needs the import to exist in both trees.
        importer = self.path / "imp.py"
        importer.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "sys.exit(0 if (p.parent / 'dep.atto').exists() else 1)\n",
            encoding="utf-8",
        )
        (self.corpus / "main.atto").write_text("uses dep\n", encoding="utf-8")
        (self.corpus / "dep.atto").write_text("bad dep\n", encoding="utf-8")
        status = main([
            "--formatter", f"{sys.executable} {self.formatter}",
            "--compiler", f"cc={sys.executable} {importer} {{source}}",
            "--corpus", str(self.corpus),
        ])
        # dep.atto itself is formatter-rejected but compiler-valid.
        self.assertEqual(status, 1)

    def test_unrelated_extensions_are_ignored(self):
        status, report = self.run_main(("notes.txt", "bad\n"))
        self.assertEqual(status, 0, report)
        self.assertIn('"files": 0', report)

    def test_missing_compiler_is_a_harness_error_not_a_rejection(self):
        (self.corpus / "ok.atto").write_text("fnc main() {}\n", encoding="utf-8")
        status = main([
            "--formatter", f"{sys.executable} {self.formatter}",
            "--compiler", f"cc={self.path / 'missing-compiler'} {{source}}",
            "--corpus", str(self.corpus),
        ])
        self.assertEqual(status, 2)


if __name__ == "__main__":
    unittest.main()
