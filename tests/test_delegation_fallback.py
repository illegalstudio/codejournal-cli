import json
import sys
import unittest
from delegation_support import DelegationCase


@unittest.skipUnless(sys.platform == 'linux', 'Linux Bubblewrap fallback')
class DelegationFallbackTest(DelegationCase):
    def setUp(self):
        super().setUp()
        helpers = self.root / 'state/codejournal/helpers'
        helpers.parent.mkdir(parents=True)
        helpers.write_text('Synthetic unavailable bundled-helper storage')

    def backend(self, body):
        path = self.bin / 'bwrap'
        path.write_text(f'#!{sys.executable}\nimport os, sys, time\n' + body)
        path.chmod(0o755)

    def assert_unprotected(self, snapshot, reason):
        self.assertEqual(snapshot['session']['status'], 'completed', json.dumps(snapshot))
        self.assertEqual(snapshot['session']['mode'], 'read-only')
        self.assertFalse(snapshot['sandbox']['enforced'])
        self.assertEqual(snapshot['sandbox']['status'], 'disabled')
        self.assertIn(reason, snapshot['sandbox']['reason'])

    def test_rejected_namespaces_continue_and_expose_actual_protection(self):
        self.backend("print('synthetic namespace denial', file=sys.stderr)\nraise SystemExit(1)\n")
        result = self.invoke('start', 'codex', '--prompt', 'Inspect only')
        self.assertIn('without filesystem sandbox', result.stderr)
        identifier = json.loads(result.stdout)['id']
        snapshot = self.settled(identifier)
        self.assert_unprotected(snapshot, 'preflight failed')
        self.assertTrue(any(e['kind'] == 'sandbox' for e in snapshot['events']))

    def test_synchronous_run_continues_with_warning_and_correct_prompt(self):
        self.backend("raise SystemExit(1)\n")
        fake = self.bin / 'codex'
        fake.write_text(f'#!{sys.executable}\nimport sys\nprompt=sys.stdin.read()\nassert "\\\"enforced\\\":false" in prompt\nassert "read-only is an instruction" in prompt\nprint("unsandboxed review")\n')
        result = self.invoke('run', 'codex', '--prompt', 'Inspect only')
        self.assertIn('unsandboxed review', result.stdout)
        self.assertIn('without filesystem sandbox', result.stderr)

    def test_preflight_timeout_continues_without_hanging(self):
        self.backend('time.sleep(20)\n')
        identifier = self.start()
        self.assert_unprotected(self.settled(identifier), 'preflight timed out')

    def test_failure_after_successful_preflight_does_not_block_provider(self):
        self.backend("if sys.argv[-1] == '--version':\n raise SystemExit(0)\nprint('synthetic launch denial',file=sys.stderr)\nraise SystemExit(1)\n")
        identifier = self.start()
        self.assert_unprotected(self.settled(identifier), 'before starting the provider')

    def test_provider_failure_is_not_retried_without_sandbox(self):
        repo = self.repository()
        fake = self.bin / 'codex'
        fake.write_text(f'#!{sys.executable}\nfrom pathlib import Path\np=Path({str(self.root / "codex" / "runs.txt")!r})\np.parent.mkdir(exist_ok=True)\np.write_text(p.read_text()+"run\\n" if p.exists() else "run\\n")\nraise SystemExit(7)\n')
        identifier = self.start(target='codex', cwd=repo)
        snapshot = self.settled(identifier)
        self.assertEqual(snapshot['session']['status'], 'stalled')
        self.assertTrue(snapshot['sandbox']['enforced'])
        self.assertEqual((self.root / 'codex/runs.txt').read_text(), 'run\n')

    def test_dry_run_does_not_execute_preflight_or_create_state(self):
        marker = self.root / 'preflight-ran'
        self.backend(f'open({str(marker)!r},"w").close()\nraise SystemExit(1)\n')
        result = self.call('start', 'codex', '--prompt', 'Inspect', '--dry-run')
        self.assertEqual(result['sandbox']['status'], 'not_checked')
        self.assertIsNone(result['sandbox']['enforced'])
        self.assertFalse(marker.exists())
        self.assertFalse((self.root / 'state/codejournal/delegations').exists())

    def test_nonexecutable_backend_continues(self):
        self.backend('raise SystemExit(1)\n')
        (self.bin / 'bwrap').chmod(0o600)
        self.assert_unprotected(self.settled(self.start()), 'cannot start sandbox preflight')

    def test_late_backend_diagnostics_do_not_expose_attempt_credentials(self):
        token_file = self.root / 'synthetic-attempt-token'
        self.backend(f"if sys.argv[-1] == '--version':\n raise SystemExit(0)\ntoken=os.environ['CJ_DELEGATION_TOKEN']\nopen({str(token_file)!r},'w').write(token)\nprint('denied '+token,file=sys.stderr)\nraise SystemExit(1)\n")
        result = self.invoke('start', 'codex', '--prompt', 'Inspect only')
        snapshot = self.settled(json.loads(result.stdout)['id'])
        token = token_file.read_text()
        self.assertTrue(token)
        self.assertNotIn(token, result.stdout+result.stderr+json.dumps(snapshot))
        self.assert_unprotected(snapshot, 'before starting the provider')

    def test_recovery_reports_current_attempt_protection(self):
        identifier = self.start('recover')
        first = self.settled(identifier)
        self.assertTrue(first['sandbox']['enforced'])
        self.backend('raise SystemExit(1)\n')
        self.call('recover', identifier, behavior='recover')
        current = self.settled(identifier)
        self.assert_unprotected(current, 'preflight failed')
        self.assertEqual(current['sandbox']['attempt'], 2)

    def test_unlinking_a_launch_marker_does_not_repeat_a_started_provider(self):
        repo = self.repository()
        fake = self.bin / 'codex'
        counter = self.root / 'codex/runs.txt'
        body = f'#!{sys.executable}\nfrom pathlib import Path\nimport os\np=Path({str(counter)!r})\np.write_text(p.read_text()+"run\\n" if p.exists() else "run\\n")\nfor marker in Path(os.environ["TMPDIR"]).parent.glob("provider-start-*"): marker.unlink()\nraise SystemExit(7)\n'
        fake.write_text(body)
        snapshot = self.settled(self.start(target='codex', cwd=repo))
        self.assertEqual(snapshot['status'], 'stalled')
        self.assertTrue(snapshot['sandbox']['enforced'])
        self.assertEqual(counter.read_text(), 'run\n')
