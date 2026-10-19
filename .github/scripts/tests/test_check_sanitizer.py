"""Exercise sanitizer orchestration failures without invoking a real compiler."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / '.github/scripts/check-sanitizer.sh'


class SanitizerChecks(unittest.TestCase):
    def run_sanitizer(self, cargo_body, expected_status, expected_text=''):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / 'bin'
            binary.mkdir()
            for name, body in {
                'cargo': cargo_body,
                'llvm-symbolizer': 'exit 0',
            }.items():
                path = binary / name
                path.write_text('#!/bin/bash\nset -eu\n' + body + '\n')
                path.chmod(0o755)
            environment = dict(os.environ, PATH=f'{binary}{os.pathsep}{os.environ["PATH"]}',
                               IPE_SANITIZER_REPORT_ROOT=str(root / 'reports'))
            result = subprocess.run(['bash', str(SCRIPT), 'thread'], cwd=root,
                                    env=environment, capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, expected_status, result.stdout + result.stderr)
            self.assertIn(expected_text, result.stdout)
            reports = [p for p in (root / 'reports').glob('thread.*') if (p / 'cargo.log').is_file()]
            self.assertEqual(len(reports), 1)
            self.assertTrue((reports[0] / 'cargo.log').is_file())
            self.assertTrue((reports[0] / 'exit-status.txt').is_file())
            return result.stdout, {p.name: p.read_text() for p in reports[0].iterdir()}

    def test_hidden_report_is_printed_and_preserved_after_exit_66(self):
        output, files = self.run_sanitizer('''
echo 'test result: ok. 354 passed; 0 failed'
report_path="${TSAN_OPTIONS#log_path=}"
printf 'WARNING: ThreadSanitizer: data race\\n' > "$report_path.123"
exit 66
''', 1, 'WARNING: ThreadSanitizer: data race')
        self.assertEqual(files['exit-status.txt'], '66\n')
        self.assertIn('tsan-report.123', files)

    def test_success_counts_never_override_failure(self):
        for message in ['test result: ok. 354 passed; 0 failed',
                        'could not compile tower', 'failed to read suppressions file']:
            with self.subTest(message=message):
                _, files = self.run_sanitizer(f"echo '{message}'\nexit 66", 1)
                self.assertEqual(files['exit-status.txt'], '66\n')

    def test_file_finding_fails_even_if_command_exits_zero(self):
        self.run_sanitizer('''
report_path="${TSAN_OPTIONS#log_path=}"
echo 'WARNING: ThreadSanitizer: data race' > "$report_path.123"
exit 0
''', 1, 'findings=1')

    def test_clean_run_succeeds_without_reusing_old_reports(self):
        self.run_sanitizer('''
mkdir -p "$IPE_SANITIZER_REPORT_ROOT/thread.old"
echo 'WARNING: ThreadSanitizer: old race' > "$IPE_SANITIZER_REPORT_ROOT/thread.old/tsan-report.999"
echo 'test result: ok. 54 passed; 0 failed'
exit 0
''', 0, 'PASS: sanitizer command and report checks passed')
