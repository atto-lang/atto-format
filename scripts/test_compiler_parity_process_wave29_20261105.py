"""Source-owned host process regressions, NOT Java/LLVM compiler parity proof."""
import sys
import unittest
from pathlib import Path

from check_compiler_parity import command_argv, execute, run_process


class CompilerParityProcessWave29Tests(unittest.TestCase):
    def test_template_requires_source_and_replaces_all_explicit_occurrences(self):
        with self.assertRaisesRegex(ValueError, r"must contain"):
            command_argv("compiler --check", Path("input.ores"))
        argv = command_argv("compiler {source} --compare={source}", Path("input.ores"))
        self.assertEqual(argv, ["compiler", "input.ores", "--compare=input.ores"])

    def test_oversized_formatter_stdout_is_not_a_successful_process(self):
        status, returncode, stdout, _ = run_process(
            [sys.executable, "-c", "import sys; sys.stdout.buffer.write(b'x'*131072)"],
            3,
            stdout_limit=32,
        )
        self.assertEqual(status, "output_limit")
        self.assertIsNotNone(returncode)
        self.assertLessEqual(len(stdout), 32)

    def test_bounded_diagnostics_do_not_turn_compiler_usage_into_rejection(self):
        status, diagnostics = execute(
            [sys.executable, "-c",
             "import sys; sys.stderr.write('e'*16000); sys.exit(2)"],
            3,
        )
        self.assertEqual(status, "usage")
        self.assertEqual(len(diagnostics), 4000)

    def test_timeout_is_not_compiler_rejection(self):
        status, _ = execute(
            [sys.executable, "-c", "import time; time.sleep(5)"],
            0.15,
        )
        self.assertEqual(status, "timeout")


if __name__ == "__main__":
    unittest.main()
