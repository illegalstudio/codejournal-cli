import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class LogAmendmentTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    @staticmethod
    def amend(handler):
        body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
        Handler.calls.append(('PATCH', handler.path, body))
        handler.reply(200, {'log': {'id': RESOURCE, 'revision': body.get('based_on', 1) + 1}})

    def test_body_status_and_optional_refs_use_the_protected_endpoint(self):
        with patch.object(Handler, 'do_PATCH', self.amend):
            result = self.cli('--json', 'log', 'update', RESOURCE[:8], '--base', '3',
                              '--body-file', '-', '--status', 'done', stdin='Late CI passed.\n')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)['log']['revision'], 4)
            self.assertEqual(Handler.calls[-1], ('PATCH', '/api/v1/tenants/demo/logs/aaaaaaaa/amend',
                                                {'based_on': 3, 'body': 'Late CI passed.', 'status': 'done'}))
            result = self.cli('log', 'update', RESOURCE, '--base', '4', '--body', '', '--clear-refs')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(Handler.calls[-1][2], {'based_on': 4, 'body': '', 'refs': []})

    def test_legacy_refs_only_updates_keep_the_existing_endpoint(self):
        with patch.object(Handler, 'do_PATCH', self.amend):
            result = self.cli('log', 'update', RESOURCE, '--ref', 'path:src/cache.rs')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('Updated refs', result.stdout)
            self.assertEqual(Handler.calls[-1], ('PATCH', '/api/v1/tenants/demo/logs/' + RESOURCE,
                                                {'refs': [{'kind': 'path', 'value': 'src/cache.rs'}]}))

    def test_invalid_modes_and_budgets_never_send_or_queue(self):
        for args in [[], ['--base', '1'], ['--body', 'x'], ['--status', 'done'],
                     ['--base', '0', '--body', 'x'], ['--base', '1', '--status', 'invalid']]:
            result = self.cli('log', 'update', RESOURCE, *args)
            self.assertNotEqual(result.returncode, 0)
        result = self.cli('log', 'update', RESOURCE, '--base', '1', '--body-file', '-', stdin='é' * 100001)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('work log body exceeds', result.stderr)
        self.assertEqual(Handler.calls, [])
        self.assertFalse(list((self.base / 'state').rglob('requests/*.json')))

    def test_offline_amendments_keep_the_original_base_and_conflicts_remain_pending(self):
        result = self.cli('--json', '--offline', 'log', 'update', RESOURCE, '--base', '2', '--status', 'done')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)['queued'])
        files = list((self.base / 'state').rglob('requests/*.json'))
        self.assertEqual(len(files), 1)
        original = files[0].read_bytes()
        queued = json.loads(original)
        self.assertTrue(queued['path'].endswith('/amend'))
        self.assertEqual(queued['body'], {'based_on': 2, 'status': 'done'})
        with patch.object(Handler, 'do_PATCH', lambda h: h.reply(409, {'message': 'Work log changed; review the current revision'})):
            failed = self.cli('sync')
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn('Work log changed', failed.stderr)
        self.assertEqual(files[0].read_bytes(), original)

    def test_unconfirmed_amendment_fails_and_show_exposes_revision(self):
        result = self.cli('log', 'update', RESOURCE, '--base', '2', '--status', 'done')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('did not confirm', result.stderr)
        with patch.object(Handler, 'do_GET', lambda h: h.reply(200, {'log': {'id': RESOURCE, 'revision': 7, 'body': 'Result'}})):
            self.assertIn('use --base 7', self.cli('log', 'show', RESOURCE).stdout)
            self.assertEqual(self.cli('log', 'show', RESOURCE, '--body').stdout, 'Result\n')
