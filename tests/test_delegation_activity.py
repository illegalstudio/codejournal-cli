from delegation_support import DelegationCase


class DelegationActivityTest(DelegationCase):
    def test_dry_runs_preview_streamed_delegations_and_plain_synchronous_runs(self):
        managed = self.call('start', 'grok', '--prompt', 'Synthetic review', '--dry-run')['args']
        direct = self.call('run', 'grok', '--prompt', 'Synthetic review', '--dry-run')['args']
        self.assertEqual(managed[managed.index('--output-format') + 1], 'streaming-json')
        self.assertIn('--include-partial-messages', managed)
        self.assertEqual(direct[direct.index('--output-format') + 1], 'plain')
        self.assertNotIn('--include-partial-messages', direct)
        self.assertFalse((self.root / 'state/codejournal/delegations/delegations.sqlite3').exists())

    def test_streaming_grok_progress_keeps_a_review_alive_without_protocol_statuses(self):
        ident = self.call('start', 'grok', '--prompt', 'Synthetic streamed review',
                          '--stall-timeout', '1.5', behavior='streaming_review')['session']['id']
        result = self.settled(ident)
        self.assertEqual(result['session']['status'], 'completed')
        self.assertEqual(result['session']['result'], 'Review completed with streamed activity')
        self.assertFalse(any(event['kind'] == 'status' for event in result['events']))

    def test_supervisor_heartbeats_alone_do_not_keep_a_silent_guest_alive(self):
        ident = self.call('start', 'grok', '--prompt', 'Synthetic silent guest',
                          '--stall-timeout', '1.5', behavior='sleep')['session']['id']
        result = self.settled(ident)
        self.assertEqual(result['session']['status'], 'stalled')
        self.assertIn('no activity', result['session']['error'])
        self.assertGreater(result['session']['guest_heartbeat'], result['session']['last_activity'])
