"""Offline harness self-tests; these are NOT Java/LLVM compiler evidence."""
import hashlib
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
        # Resolve once: macOS temp dirs live under the /var -> /private/var
        # system symlink, which snapshot_corpus refuses as an ancestor.
        self.path = Path(self.root.name).resolve()
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
        self.assertEqual(json.loads(report)["errors"], [])
        self.assertEqual(json.loads(report)["findings"], [])

    def test_meaning_change_is_reported_as_newly_accepted(self):
        status, report = self.run_main(
            ("m.atto", "val x; write-to-val\n"),
            ("baseline.atto", "fnc main() {}\n"),
        )
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
        (self.corpus / "baseline.atto").write_text("fnc main() {}\n", encoding="utf-8")
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
            "if '!!!' in p.read_text(): sys.exit(1)\n"
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
            "t=p.read_text()\n"
            "sys.exit(0 if (root/'.ores.pkg.toml').is_file() and "
            "(root/'config.json').is_file() and '!!!' not in t else 1)\n",
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
        errors = json.loads(report)["errors"]
        self.assertEqual(len(errors), 1)
        self.assertIn("no original sources accepted", errors[0]["message"])

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

    @unittest.skipUnless(os.name == "posix", "requires POSIX symlinks")
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
        (self.corpus / "resource.bin").write_bytes(b"12345678901234567")
        # Source fits: only the 17-byte auxiliary resource violates the cap.
        with mock.patch("check_formatter_acceptance_delta.MAX_SOURCE_BYTES", 16):
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

    @unittest.skipUnless(os.name == "posix", "requires POSIX symlinks")
    def test_snapshot_refuses_symlinked_ancestor(self):
        alias = self.path / "linked_parent"
        alias.symlink_to(self.path, target_is_directory=True)
        nested_root = alias / "corpus"
        # The final directory is ordinary; only an ancestor is linked.
        self.assertFalse(nested_root.is_symlink())
        with self.assertRaisesRegex(ValueError, "symlinked corpus root/ancestor"):
            snapshot_corpus(nested_root, self.path / "snapshot")
        (self.corpus / "main.atto").write_text("fnc main() {}\n", encoding="utf-8")
        report = self.path / "alias_report.json"
        status = main([
            "--formatter", f"{sys.executable} {self.formatter}",
            "--compiler", f"cc={sys.executable} {self.compiler} {{source}}",
            "--corpus", str(nested_root),
            "--report", str(report),
        ])
        # The operator-supplied path is resolved once at argument parsing and
        # the real root is what gets audited and recorded; the snapshot layer
        # above still refuses a symlinked component of the path it is given.
        self.assertEqual(status, 0)
        roots = json.loads(report.read_text())["summary"]["corpus_roots"]
        self.assertEqual(roots, [str(self.corpus)])

    def test_symlinked_corpus_root_itself_is_refused(self):
        (self.corpus / "main.atto").write_text("fnc main() {}\n", encoding="utf-8")
        link = self.path / "corpus_link"
        link.symlink_to(self.corpus, target_is_directory=True)
        with self.assertRaises(SystemExit):
            main([
                "--formatter", f"{sys.executable} {self.formatter}",
                "--compiler", f"cc={sys.executable} {self.compiler} {{source}}",
                "--corpus", str(link),
            ])

    @unittest.skipUnless(hasattr(os, "mkfifo"), "requires POSIX FIFO")
    def test_snapshot_refuses_special_files_without_opening_them(self):
        os.mkfifo(self.corpus / "named_pipe")
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)
        self.assertEqual(report, "")

    def test_formatter_only_mutates_isolated_copy(self):
        text = "val x; write-to-val\n"
        status, report = self.run_main(
            ("m.atto", text), ("baseline.atto", "fnc main() {}\n"),
        )
        self.assertEqual(status, 1, report)
        self.assertEqual((self.corpus / "m.atto").read_text(encoding="utf-8"), text)
        self.assertIn("newly-accepted:cc", report)

    def test_empty_inventory_fails_without_claiming_a_report(self):
        status, report = self.run_main(("README.md", "this is not source\n"))
        self.assertEqual(status, 2)
        self.assertEqual(report, "")


    def test_always_successful_compiler_fails_invalid_syntax_control(self):
        self.compiler.write_text("import sys\nsys.exit(0)\n", encoding="utf-8")
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)
        self.assertEqual(json.loads(report)["summary"]["harness_errors"], 1)
        self.assertFalse((self.corpus / "__oresfmt_invalid_syntax_control.atto").exists())

    def test_formatter_that_mutates_source_during_stdout_is_rejected(self):
        initial = "fnc main() {}\n"
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "original=p.read_text()\n"
            "p.write_text(original+'// silent mutation\\n')\n"
            "sys.stdout.write(original)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", initial))
        self.assertEqual(status, 2, report)
        self.assertIn("mutated-input", report)
        # Audit must never modify the original source checkout.
        self.assertEqual((self.corpus / "main.atto").read_text(), initial)

    @unittest.skipUnless(os.name == "posix", "requires POSIX process signals")
    def test_compiler_terminated_by_signal_is_not_a_syntax_rejection(self):
        self.compiler.write_text(
            "import os\nimport signal\nos.kill(os.getpid(), signal.SIGTERM)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2)
        summary = json.loads(report)["summary"]
        self.assertGreater(summary["harness_errors"], 0)
        self.assertEqual(summary["baseline_accepted_by_compiler"]["cc"], 0)

    def test_canary_is_absent_from_baseline_inventory_and_diff_trees(self):
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 0, report)
        self.assertIn('"files": 1', report)
        self.assertNotIn("__oresfmt_invalid_syntax_control.atto", report)
        self.assertFalse((self.corpus / "__oresfmt_invalid_syntax_control.atto").exists())


    def test_manifest_only_compiler_cannot_fake_invalid_syntax_rejection(self):
        (self.corpus / "src").mkdir()
        (self.corpus / "src" / "main.atto").write_text(
            "fnc main() {}\n", encoding="utf-8")
        (self.corpus / ".ores.pkg.toml").write_text(
            "[package]\nname = 'demo'\n", encoding="utf-8")
        # This stub accepts every source in a valid package without parsing.
        # A control outside that package would be rejected and miss the flaw.
        self.compiler.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "root=p.parent.parent\n"
            "sys.exit(0 if (root/'.ores.pkg.toml').is_file() else 1)\n",
            encoding="utf-8",
        )
        status, report = self.run_main()
        self.assertEqual(status, 2, report)
        errors = json.loads(report)["errors"]
        self.assertTrue(any("invalid syntax control was accept" in entry["message"]
                            for entry in errors), errors)
        self.assertEqual((self.corpus / "src" / "main.atto").read_text(),
                         "fnc main() {}\n")


    def test_cross_file_manifest_mutation_is_detected_and_cannot_pass(self):
        initial = "fnc main() {}\n"
        manifest = "[package]\nname = 'demo'\n"
        (self.corpus / ".ores.pkg.toml").write_text(manifest, encoding="utf-8")
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "t=p.read_text()\n"
            "(p.parent/'.ores.pkg.toml').write_text('tampered by formatter\\n')\n"
            "sys.stdout.write(t)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", initial))
        self.assertEqual(status, 2, report)
        self.assertIn("original input modified during audit", report)
        self.assertEqual((self.corpus / "main.atto").read_text(), initial)
        self.assertEqual((self.corpus / ".ores.pkg.toml").read_text(), manifest)

    def test_snapshot_hash_manifest_detects_auxiliary_payload_changes(self):
        (self.corpus / "main.atto").write_text("fnc main() {}\n")
        binary = self.corpus / "payload.bin"
        binary.write_bytes(b"\x00\xff\xaa")
        dest = self.path / "original_snapshot"
        hashes = {}
        self.assertEqual(snapshot_corpus(self.corpus, dest, hashes), 2)
        self.assertEqual(len(hashes), 2)
        binary_snapshot = dest / "payload.bin"
        self.assertIn(binary_snapshot, hashes)
        self.assertEqual(hashes[binary_snapshot],
                         hashlib.sha256(binary.read_bytes()).digest())
        binary_snapshot.write_bytes(b"\x00\xff\xab")
        self.assertNotEqual(hashes[binary_snapshot],
                            hashlib.sha256(binary_snapshot.read_bytes()).digest())


    @unittest.skipUnless(os.name == "posix", "requires POSIX process signals")
    def test_crashing_formatter_is_harness_error_not_valid_rejection(self):
        self.formatter.write_text(
            "import os\nimport signal\nos.kill(os.getpid(), signal.SIGTERM)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2, report)
        results = json.loads(report)["results"]
        self.assertEqual(results[0]["format"], "crashed")
        self.assertGreater(json.loads(report)["summary"]["harness_errors"], 0)


    def test_formatted_manifest_mutation_fails_closed(self):
        original = "[package]\nname = 'fixture'\n"
        (self.corpus / ".ores.pkg.toml").write_text(original, encoding="utf-8")
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "t=p.read_text()\n"
            "if '/fmt/' in str(p):\n"
            " (p.parent/'.ores.pkg.toml').write_text('changed manifest\\n')\n"
            "sys.stdout.write(t)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2, report)
        self.assertTrue(any(
            "formatted auxiliary input modified during audit" in error["message"]
            for error in json.loads(report)["errors"]
        ))
        self.assertEqual((self.corpus / ".ores.pkg.toml").read_text(), original)

    def test_formatter_cannot_create_hidden_auxiliary_config(self):
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "t=p.read_text()\n"
            "if '/fmt/' in str(p):\n"
            " (p.parent/'unexpected.toml').write_text('[package]\\nname=\"bad\"\\n')\n"
            "sys.stdout.write(t)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2, report)
        self.assertTrue(any(
            "formatted unexpected/replaced file during audit" in error["message"]
            for error in json.loads(report)["errors"]
        ))
        self.assertFalse((self.corpus / "unexpected.toml").exists())

    def test_invalid_scheduler_budgets_rejected(self):
        (self.corpus / "main.atto").write_text("fnc main() {}\n", encoding="utf-8")
        for option, invalid in [
            ("--jobs", "0"), ("--jobs", "-1"),
            ("--timeout", "0"), ("--timeout", "-10"),
        ]:
            with self.subTest(option=option, invalid=invalid):
                with self.assertRaises(SystemExit) as caught:
                    main(["--formatter", f"{sys.executable} {self.formatter}",
                          "--compiler", f"cc={sys.executable} {self.compiler} {{source}}",
                          "--corpus", str(self.corpus), option, invalid])
                self.assertEqual(caught.exception.code, 2)


    def test_compiler_admission_waits_for_all_formatted_imports(self):
        # A former single-pass worker pool could compile fast.atto against
        # a not-yet-formatted slow.atto and report false rejection.
        (self.corpus / "fast.atto").write_text("unformatted fast\n", encoding="utf-8")
        (self.corpus / "slow.atto").write_text("unformatted slow\n", encoding="utf-8")
        self.formatter.write_text(
            "import sys\nimport time\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "if p.name == 'slow.atto': time.sleep(0.2)\n"
            "sys.stdout.write(p.read_text().replace('unformatted', 'formatted'))\n",
            encoding="utf-8",
        )
        self.compiler.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "if p.name.startswith('__oresfmt_invalid_syntax_control'): sys.exit(1)\n"
            "if '/fmt/' in str(p):\n"
            " peer=p.parent/('slow.atto' if p.name=='fast.atto' else 'fast.atto')\n"
            " if 'unformatted' in peer.read_text(): sys.exit(1)\n"
            "sys.exit(0)\n",
            encoding="utf-8",
        )
        status, report = self.run_main()
        self.assertEqual(status, 0, report)
        self.assertEqual(json.loads(report)["summary"]["files"], 2)
        self.assertEqual(json.loads(report)["findings"], [])

    def test_added_formatted_source_cannot_change_project_unnoticed(self):
        self.formatter.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "t=p.read_text()\n"
            "if '/fmt/' in str(p):\n"
            " (p.parent/'unexpected_import.atto').write_text('fnc injected() {}\\n')\n"
            "sys.stdout.write(t)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2, report)
        self.assertTrue(any(
            "formatted unexpected/replaced file during audit" in error["message"]
            for error in json.loads(report)["errors"]
        ))
        self.assertFalse((self.corpus / "unexpected_import.atto").exists())

    def test_compiler_added_source_to_original_tree_is_harness_error(self):
        self.compiler.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "if p.name.startswith('__oresfmt_invalid_syntax_control'): sys.exit(1)\n"
            "if '/orig/' in str(p):\n"
            " (p.parent/'unexpected_side_effect.atto').write_text('fnc unexpected() {}\\n')\n"
            "sys.exit(0)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2, report)
        self.assertTrue(any(
            "original unexpected/replaced file during audit" in error["message"]
            for error in json.loads(report)["errors"]
        ))
        self.assertFalse((self.corpus / "unexpected_side_effect.atto").exists())


    def test_compiler_mutating_formatted_source_fails_closed(self):
        # The formatted source is supposed to be immutable between the
        # formatter phase and the final report. The old audit checked only
        # original-tree hashes and non-source files on the formatted side.
        self.compiler.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "if p.name.startswith('__oresfmt_invalid_syntax_control'): sys.exit(1)\n"
            "if '/fmt/' in str(p): p.write_text(p.read_text()+'// tampered\\n')\n"
            "sys.exit(0)\n",
            encoding="utf-8",
        )
        original = "fnc main() {}\n"
        status, report = self.run_main(("main.atto", original))
        self.assertEqual(status, 2, report)
        self.assertTrue(any(
            "formatted source modified by compiler" in error["message"]
            for error in json.loads(report)["errors"]
        ))
        self.assertEqual((self.corpus / "main.atto").read_text(), original)


    def test_invalid_syntax_control_cannot_tamper_with_package_after_integrity(self):
        # Control runs must be included in the final integrity scan. A
        # malicious or buggy compiler can reject the negative control while
        # changing the package manifest, so rejection alone is not evidence.
        initial = "[package]\nname = 'fixture'\n"
        (self.corpus / ".ores.pkg.toml").write_text(initial, encoding="utf-8")
        self.compiler.write_text(
            "import sys\nfrom pathlib import Path\n"
            "p=Path(sys.argv[-1])\n"
            "if p.name == '__oresfmt_invalid_syntax_control.atto':\n"
            " (p.parent/'.ores.pkg.toml').write_text('modified by control\\n')\n"
            " sys.exit(1)\n"
            "sys.exit(0)\n",
            encoding="utf-8",
        )
        status, report = self.run_main(("main.atto", "fnc main() {}\n"))
        self.assertEqual(status, 2, report)
        errors = json.loads(report)["errors"]
        self.assertTrue(any("original input modified during audit" in row["message"]
                            for row in errors), errors)
        self.assertEqual((self.corpus / ".ores.pkg.toml").read_text(), initial)


if __name__ == "__main__":
    unittest.main()
