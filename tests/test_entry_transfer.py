import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class EntryTransferTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'
        def transfer(handler):
            body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
            Handler.calls.append(('POST', handler.path, body))
            handler.reply(200, {'id': RESOURCE, 'from': 'fixture', 'to': body['to'],
                                'dry_run': body['dry_run'], 'refs': []})
        self.enterContext(patch.object(Handler, 'do_POST', transfer))

    def test_move_preserves_literal_prefix_flags_and_preview_is_never_queued(self):
        result = self.cli('--json', 'entry', 'move', RESOURCE, '--to', 'destination',
                          '--path-prefix', 'apps/tool', '--replace-prefix', '', '--dry-run')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)['dry_run'])
        method, path, body = Handler.calls[-1]
        self.assertEqual((method, path), ('POST', '/api/v1/tenants/demo/entries/' + RESOURCE + '/move'))
        self.assertEqual(body['path_prefix'], 'apps/tool')
        self.assertEqual(body['replace_prefix'], '')
        self.assertFalse(list((self.base / 'state').rglob('requests/*.json')))
        Handler.calls.clear()
        offline = self.cli('--offline', 'entry', 'move', RESOURCE, '--to', 'destination', '--dry-run')
        self.assertNotEqual(offline.returncode, 0)
        self.assertEqual(Handler.calls, [])
        self.assertFalse(list((self.base / 'state').rglob('requests/*.json')))

    def test_offline_move_is_durable_and_unpaired_rewrite_flags_are_rejected(self):
        rejected = self.cli('entry', 'move', RESOURCE, '--to', 'destination', '--path-prefix', 'apps/tool')
        self.assertNotEqual(rejected.returncode, 0)
        self.assertEqual(Handler.calls, [])
        queued = self.cli('--offline', '--json', 'entry', 'move', RESOURCE, '--to', 'destination')
        self.assertEqual(queued.returncode, 0, queued.stderr)
        self.assertTrue(json.loads(queued.stdout)['queued'])
        pending = list((self.base / 'state').rglob('requests/*.json'))
        self.assertEqual(len(pending), 1)
        self.assertEqual(json.loads(pending[0].read_text())['body']['to'], 'destination')
