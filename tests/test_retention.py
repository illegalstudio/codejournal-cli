import json
import uuid
from unittest.mock import patch

from auto_sync_support import Handler
from hook_outbox_support import HookOutboxCase


class RetentionTest(HookOutboxCase):
    def test_durable_override_and_explicit_note_use_reach_the_expected_routes(self):
        entry = self.cli('add', '--durable', '--kind', 'howto', '--title', 'Stable observation', '--body', 'Synthetic')
        note = entry['entry']['id']
        self.cli('entry', 'use', note, 'pr', 'example/project#42')
        self.assertTrue(Handler.calls[0][2]['durable'])
        self.assertTrue(Handler.calls[0][1].endswith('/entries'))
        self.assertTrue(Handler.calls[1][1].endswith(f'/entries/{note}/use'))
        self.assertEqual(Handler.calls[1][2]['ref'], 'example/project#42')

    def test_invalid_and_unclassified_notifications_never_reach_the_api(self):
        for arguments, message in [
            (['--kind', 'info', '--title', 'Routine'], '--important'),
            (['--important', '--kind', 'needs_input', '--title', 'Decision'], 'invalid value'),
            (['--important', '--kind', 'error', '--title', 'é' * 256], 'exceeds 255'),
        ]:
            result = self.cli('notify', *arguments, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(message, result.stderr)
        self.assertEqual(Handler.calls, [])
        self.assertEqual(self.queue(), [])

    def test_import_sends_supported_notifications_only(self):
        path = self.base / 'notifications.jsonl'
        rows = [{'type': 'notification', 'id': str(uuid.uuid4()), 'title': 'Synthetic', 'kind': kind}
                for kind in ['needs_input', 'done']]
        rows.append({**rows[1], 'id': str(uuid.uuid4()), 'source': 'hook_approval'})
        path.write_text('\n'.join(json.dumps(row) for row in rows))
        transmitted = []

        def chunk(handler):
            body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
            transmitted.extend(body['records'])
            handler.respond(200, {})

        def finalize(handler):
            handler.rfile.read(int(handler.headers['Content-Length']))
            handler.respond(200, {'counts': {'notifications': 1, 'entries': 0, 'projects': 0, 'skipped': 0}})

        with patch.object(Handler, 'do_PUT', chunk, create=True), patch.object(Handler, 'do_POST', finalize):
            result = self.cli('import', str(path))
        self.assertEqual([(row['id'], row['kind']) for row in transmitted], [(rows[1]['id'], 'done')])
        self.assertEqual(result['skipped'], 2)

    def test_legacy_input_notification_is_skipped_before_the_next_write(self):
        retired = self.enqueue('Legacy intervention request')['request_id']
        path = self.queue()[0]
        old = json.loads(path.read_text())
        old['path'] = '/api/v1/tenants/demo/notifications'
        old['body'] = {'title': 'Legacy question', 'kind': 'needs_input'}
        path.write_text(json.dumps(old))
        fresh = self.enqueue('New synthetic note')['request_id']
        result = self.cli('sync', check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(retired, result.stderr)
        self.assertEqual([call[0] for call in Handler.calls], [fresh])
        self.assertEqual(self.queue(), [])

    def test_expired_hook_does_not_block_other_events_in_the_same_batch(self):
        old_path, old = self.save(self.origin())
        fresh_path, fresh = self.save(self.origin())

        def partial(handler):
            body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
            self.assertEqual({item['id'] for item in body['events']}, {old['id'], fresh['id']})
            handler.respond(200, {'acknowledged': [fresh['id']], 'expired': [old['id']]})

        with patch.object(Handler, 'do_POST', partial):
            result = self.cli('sync', check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(old['id'] + ' expired', result.stderr)
        self.assertEqual(json.loads(result.stdout)['expired_hooks'], 1)
        self.assertFalse(old_path.exists())
        self.assertFalse(fresh_path.exists())

    def test_temporary_add_uses_a_dedicated_server_route(self):
        result = self.cli('add', '--temporary', '--kind', 'discovery', '--title', 'Recent observation', '--body', 'Synthetic')
        self.assertTrue(result['entry']['id'])
        self.assertEqual(Handler.calls[-1][1], '/api/v1/tenants/demo/projects/fixture/entries/temporary')
