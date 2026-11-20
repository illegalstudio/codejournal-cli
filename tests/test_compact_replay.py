import json
import time
from unittest.mock import patch

from auto_sync_support import AutoSyncCase, Handler


class CompactReplayTest(AutoSyncCase):
    def test_confirmed_compact_response_acknowledges_only_the_matching_queued_write(self):
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'
        first = self.enqueue('First write')['request_id']
        second = self.enqueue('Second write')['request_id']
        original = Handler.do_POST
        headers = []

        def send(handler):
            key = handler.headers['Idempotency-Key']
            headers.append(handler.headers.get('Idempotency-Replay'))
            if key != first:
                return original(handler)
            body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
            Handler.calls.append((key, handler.path, body, time.monotonic()))
            handler.respond(200, {'replay': 'compact-v1', 'request_id': first, 'applied': True,
                                  'result': {'entry': {'id': first}}})

        with patch.object(Handler, 'do_POST', send):
            result = self.cli('sync')
        self.assertTrue(result['synced'])
        self.assertEqual([key for key, *_ in Handler.calls], [first, second])
        self.assertEqual(headers, ['compact-v1', 'compact-v1'])
        self.assertEqual(self.queue(), [])

    def test_acknowledgement_for_a_different_request_preserves_the_queue(self):
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'
        first = self.enqueue()['request_id']
        second = self.enqueue()['request_id']

        def send(handler):
            handler.rfile.read(int(handler.headers['Content-Length']))
            handler.respond(200, {'replay': 'compact-v1', 'request_id': second, 'applied': True, 'result': {}})

        with patch.object(Handler, 'do_POST', send):
            refused = self.cli('outbox', 'retry', first, check=False)
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn('invalid compact replay acknowledgement', refused.stderr)
        self.assertEqual(len(self.queue()), 2)
