"""Prevent a successful merge gate from concealing missing or failed CI."""
import json
import os
from pathlib import Path
import subprocess
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[3]
WORKFLOWS = ROOT / '.github/workflows'
GATES = ['ci.yml', 'security.yml', 'coverage.yml', 'bench.yml']


def load(path):
    # BaseLoader preserves the GitHub Actions `on` key as text (YAML 1.2).
    return yaml.load(path.read_text(), Loader=yaml.BaseLoader)


class MergeQueueChecks(unittest.TestCase):
    def test_gate_checks_all_jobs_and_rejects_non_success(self):
        for name in GATES:
            jobs = load(WORKFLOWS / name)['jobs']
            gate = jobs['queue-gate']
            with self.subTest(workflow=name):
                self.assertEqual(set(gate['needs']), set(jobs) - {'queue-gate'})
                self.assertEqual(gate['if'], 'always()')
                step = gate['steps'][0]
                self.assertEqual(step['env']['JOB_RESULTS'], '${{ toJSON(needs) }}')
                for state in ['success', 'failure', 'cancelled', 'skipped', 'unknown', None]:
                    results = {job: {'result': 'success'} for job in gate['needs']}
                    results[gate['needs'][0]]['result'] = state
                    run = subprocess.run(['bash', '-e', '-c', step['run']],
                                         env=dict(os.environ, JOB_RESULTS=json.dumps(results)),
                                         capture_output=True, timeout=5)
                    self.assertEqual(run.returncode == 0, state == 'success', (name, state))
                for bad in ['{}', 'not-json']:
                    run = subprocess.run(['bash', '-e', '-c', step['run']],
                                         env=dict(os.environ, JOB_RESULTS=bad),
                                         capture_output=True, timeout=5)
                    self.assertNotEqual(run.returncode, 0)

    def test_required_checks_exist_and_run_on_all_pr_changes(self):
        config = load(ROOT / '.mergify.yml')
        queue = config['queue_rules'][0]
        prefix = 'check-success=@github-actions/'
        entry = {c[len(prefix):] for c in queue['queue_conditions'] if c.startswith(prefix)}
        merge = {c[len(prefix):] for c in queue['merge_conditions'] if c.startswith(prefix)}
        self.assertTrue(merge)
        self.assertEqual(entry, merge)
        self.assertIn('Workflow validation', merge)
        definitions = {}
        for path in WORKFLOWS.glob('*.yml'):
            workflow = load(path)
            for job in workflow.get('jobs', {}).values():
                name = job.get('name')
                if name in merge:
                    self.assertNotIn(name, definitions, f'Duplicate required check: {name}')
                    definitions[name] = path
                    self.assertIn('pull_request', workflow['on'])
                    trigger = workflow['on']['pull_request'] or {}
                    self.assertNotIn('paths', trigger)
                    self.assertNotIn('paths-ignore', trigger)
                    self.assertNotIn('types', trigger)
                    if 'branches' in trigger:
                        self.assertIn('main', trigger['branches'])
                    self.assertNotIn('branches-ignore', trigger)
                    self.assertIn(job.get('if'), [None, 'always()'])
        self.assertEqual(merge, set(definitions))

    def test_review_and_hold_conditions_have_no_auto_approval(self):
        config = load(ROOT / '.mergify.yml')
        queue = config['queue_rules'][0]
        for key in ['queue_conditions', 'merge_conditions']:
            self.assertIn('#approved-reviews-by>=1', queue[key])
            self.assertIn('#changes-requested-reviews-by=0', queue[key])
        for condition in ['base=main', '-draft', 'label!=do-not-merge', 'label!=work-in-progress']:
            self.assertIn(condition, queue['queue_conditions'])
        for rule in config.get('pull_request_rules', []):
            self.assertNotIn('review', rule.get('actions', {}))
        self.assertGreater(int(config['merge_queue']['max_parallel_checks']), 1)


if __name__ == '__main__':
    unittest.main()
