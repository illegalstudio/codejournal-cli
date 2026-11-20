from auto_sync_support import AutoSyncCase, Handler


class OutboxRetryTest(AutoSyncCase):
    def test_only_the_oldest_request_is_delivered_with_its_original_id(self):
        first = self.enqueue('First note')['request_id']
        second = self.enqueue('Second note')['request_id']
        refused = self.cli('outbox', 'retry', second[:8], check=False)
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn(first, refused.stderr)
        self.assertEqual(Handler.calls, [])
        self.assertEqual(self.cli('outbox', 'retry', first[:8]), {'retried': first})
        self.assertEqual([key for key, *_ in Handler.calls], [first])
        self.assertEqual(len(self.queue()), 1)
        self.assertIn(second, self.queue()[0].name)
        self.assertEqual(self.saved(), {})

    def test_wrong_credential_cannot_replay_a_pending_request(self):
        request = self.enqueue()['request_id']
        self.env['CJ_TOKEN'] = 'another-synthetic-account'
        refused = self.cli('outbox', 'retry', request, check=False)
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn('current credential scope', refused.stderr)
        self.assertEqual(Handler.calls, [])
        self.assertEqual(len(self.queue()), 1)

    def test_server_retry_after_is_persisted_and_does_not_reorder_requests(self):
        request = self.enqueue()['request_id']
        Handler.failures.append(429)
        Handler.retry_after = 60
        failed = self.cli('outbox', 'retry', request, check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn('remains pending', failed.stderr)
        deferred = self.cli('outbox', 'retry', request, check=False)
        self.assertNotEqual(deferred.returncode, 0)
        self.assertIn('retry delay', deferred.stderr)
        self.assertEqual([key for key, *_ in Handler.calls], [request])
        self.assertEqual(len(self.queue()), 1)
