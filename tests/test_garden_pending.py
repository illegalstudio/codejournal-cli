import json
import subprocess
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler
import test_garden_review as modern


class GardenPendingTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.enterContext(patch.object(Handler, 'do_GET', modern.read))
        self.enterContext(patch.object(Handler, 'do_POST', modern.write))

    def test_pending_json_reads_only_the_stored_queue_and_always_returns_an_envelope(self):
        result = self.cli('--json', '--project', 'fixture', 'garden', '--pending', '--limit', '1')
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertEqual(len(payload['findings']), 1)
        self.assertEqual(payload['counts']['pending'], 7)
        self.assertEqual(len(Handler.calls), 1)
        self.assertEqual(Handler.calls[0][0], 'GET')
        Handler.calls.clear()
        cached = self.cli('--offline', '--json', '--project', 'fixture', 'garden', '--pending')
        self.assertNotEqual(cached.returncode, 0)
        self.assertEqual(Handler.calls, [])

    def test_empty_pending_queue_is_structured_and_deferral_guidance_is_complete(self):
        def empty(handler):
            handler.reply(200, {'findings': [], 'counts': {'pending': 0, 'reviewed': 1, 'deferred': 0}, 'next': None})
        with patch.object(Handler, 'do_GET', empty):
            result = self.cli('--json', '--project', 'fixture', 'garden', '--pending')
            self.assertEqual(json.loads(result.stdout)['findings'], [])
            text = self.cli('--project', 'fixture', 'garden', '--pending').stdout
        self.assertIn('No pending findings', text)
        self.assertIn('--outcome deferred --until', text)

    def test_paths_present_in_another_checked_out_branch_are_reported_as_elsewhere(self):
        def git(*args):
            subprocess.run(['git', '-c', 'commit.gpgsign=false', *args], cwd=self.repo, env=self.env,
                           check=True, capture_output=True)
        git('config', 'user.email', 'synthetic@example.test')
        git('config', 'user.name', 'Synthetic')
        (self.repo / 'README.md').write_text('base')
        git('add', 'README.md')
        git('commit', '-m', 'Initial')
        other = self.base / 'review'
        git('worktree', 'add', '-b', 'review', str(other))
        (other / 'only-in-review.rs').write_text('Synthetic')
        entries = [{'id': 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', 'refs': [{'kind': 'path', 'value': 'only-in-review.rs'}]}]
        def read(handler):
            if '/garden?' in handler.path:
                return handler.reply(200, {'entries': entries, 'docs': [], 'next_page': None})
            return modern.read(handler)
        with patch.object(Handler, 'do_GET', read):
            result = self.cli('--json', '--project', 'fixture', 'garden', '--dry-run')
        self.assertEqual(result.returncode, 0, result.stderr)
        observed = next(body['observations'][0] for method, path, body in Handler.calls
                        if method == 'POST' and path.endswith('/preview') and body.get('observations'))
        self.assertEqual(observed['worktrees'], ['review'])
        self.assertFalse(observed['present'])

    def test_detached_worktrees_supply_evidence_without_a_branch_reference(self):
        def git(*args):
            return subprocess.run(['git', '-c', 'commit.gpgsign=false', *args], cwd=self.repo, env=self.env,
                                  check=True, capture_output=True, text=True).stdout.strip()
        git('config', 'user.email', 'synthetic@example.test')
        git('config', 'user.name', 'Synthetic')
        (self.repo / 'README.md').write_text('base')
        git('add', 'README.md')
        git('commit', '-m', 'Initial')
        head = git('rev-parse', 'HEAD')
        other = self.base / 'detached review'
        git('worktree', 'add', '--detach', str(other))
        (other / 'detached.rs').write_text('Synthetic uncommitted source')
        entries = [{'id': 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', 'refs': [{'kind': 'path', 'value': 'detached.rs'}]}]
        def read(handler):
            if '/garden?' in handler.path:
                return handler.reply(200, {'entries': entries, 'docs': [], 'next_page': None})
            return modern.read(handler)
        with patch.object(Handler, 'do_GET', read):
            result = self.cli('--json', '--project', 'fixture', 'garden', '--dry-run')
        self.assertEqual(result.returncode, 0, result.stderr)
        observed = next(body['observations'][0] for method, path, body in Handler.calls
                        if method == 'POST' and path.endswith('/preview') and body.get('observations'))
        self.assertEqual(observed['worktrees'], ['detached ' + head[:12]])
