import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class LogMoveTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    @staticmethod
    def move_response(handler):
        body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
        Handler.calls.append((handler.command, handler.path, body))
        handler.reply(200, {'log': {'id': RESOURCE}, 'from': 'fixture', 'to': body['to'],
                           'dry_run': body.get('dry_run', False), 'moved_logs': 2})

    def test_direct_move_and_preview_use_the_focused_endpoint(self):
        with patch.object(Handler, 'do_POST', self.move_response):
            result = self.cli('--json', 'log', 'move', RESOURCE[:8], '--to', 'target', '--dry-run')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(json.loads(result.stdout)['dry_run'])
            self.assertEqual(Handler.calls[-1], ('POST', '/api/v1/tenants/demo/logs/aaaaaaaa/move',
                                                {'to': 'target', 'dry_run': True}))
            result = self.cli('log', 'move', RESOURCE, '--to', 'target')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('Moved work log', result.stdout)
        self.assertFalse(list((self.base / 'state').rglob('requests/*.json')))

    def test_real_offline_moves_queue_but_previews_and_invalid_arguments_do_not(self):
        for args in [('log', 'move', RESOURCE, '--to', 'target', '--dry-run'),
                     ('log', 'move', 'invalid', '--to', 'target'),
                     ('log', 'move', RESOURCE, '--to', '@global')]:
            result = self.cli('--offline', *args)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(Handler.calls, [])
        self.assertFalse(list((self.base / 'state').rglob('requests/*.json')))
        result = self.cli('--json', '--offline', 'log', 'move', RESOURCE, '--to', 'target')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)['queued'])
        pending = list((self.base / 'state').rglob('requests/*.json'))
        self.assertEqual(len(pending), 1)
        self.assertEqual(json.loads(pending[0].read_text())['body'], {'to': 'target', 'dry_run': False})

    def test_plan_flag_is_explicit_and_docs_cannot_move_linked_logs(self):
        with patch.object(Handler, 'do_PATCH', self.move_response):
            for flag in ['--with-logs', '--include-linked-logs']:
                result = self.cli('plan', 'move', RESOURCE, '--to', 'target', flag)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue(Handler.calls[-1][2]['with_logs'])
                self.assertIn('2 linked work log(s)', result.stdout)
            result = self.cli('plan', 'move', RESOURCE, '--to', 'target')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn('with_logs', Handler.calls[-1][2])
        Handler.calls.clear()
        result = self.cli('doc', 'move', RESOURCE, '--to', 'target', '--with-logs')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(Handler.calls, [])

    def test_old_server_cannot_silently_claim_that_linked_logs_moved(self):
        result = self.cli('plan', 'move', RESOURCE, '--to', 'target', '--with-logs')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('did not confirm linked log transfer', result.stderr)
