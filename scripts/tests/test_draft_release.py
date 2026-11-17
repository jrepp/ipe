"""Check release boundaries with a fake gh; never create a real release."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class DraftReleaseTests(unittest.TestCase):
    def run_release(self, tag, status=0):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            command = root / 'gh'
            command.write_text('#!/usr/bin/env python3\n'
                               'import json, os, sys\n'
                               'open(os.environ["CALL_LOG"], "w").write(json.dumps(sys.argv[1:]))\n'
                               'sys.exit(int(os.environ["GH_STATUS"]))\n')
            command.chmod(0o755)
            log = root / 'call.json'
            env = dict(os.environ, PATH=f'{root}{os.pathsep}{os.environ["PATH"]}',
                       GH_REPO='fixture/repository', GH_STATUS=str(status), CALL_LOG=str(log))
            result = subprocess.run(['bash', str(ROOT / 'scripts/draft-release.sh'), tag],
                                    env=env, capture_output=True, text=True, timeout=10)
            return result, json.loads(log.read_text()) if log.exists() else None

    def test_stable_release_is_draft_with_generated_notes_and_existing_tag(self):
        result, args = self.run_release('v1.2.3')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(args, ['release', 'create', 'v1.2.3', '--repo', 'fixture/repository',
                               '--verify-tag', '--draft', '--generate-notes', '--title', 'v1.2.3'])

    def test_prerelease_is_marked_and_still_a_draft(self):
        result, args = self.run_release('v1.2.3-rc.1')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('--prerelease', args)
        self.assertIn('--draft', args)
        self.assertIn('--verify-tag', args)

    def test_invalid_tags_never_call_github(self):
        for tag in ['', 'main', 'v1', '--latest', 'v1.2.3\necho unsafe', 'v1.2.3;echo unsafe']:
            with self.subTest(tag=tag):
                result, args = self.run_release(tag)
                self.assertEqual(result.returncode, 2)
                self.assertIsNone(args)
                if tag:
                    self.assertNotIn(tag, result.stderr)

    def test_github_errors_propagate_without_edit_or_publish_fallback(self):
        for status in [1, 4, 7]:
            with self.subTest(status=status):
                result, args = self.run_release('v1.2.3', status)
                self.assertEqual(result.returncode, status)
                self.assertEqual(args[:2], ['release', 'create'])
                self.assertIn('--draft', args)
