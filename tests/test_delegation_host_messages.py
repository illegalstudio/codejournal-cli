from contextlib import closing
import sqlite3
import json
from delegation_support import DelegationCase


class DelegationHostMessagesTest(DelegationCase):
    def test_full_history_pages_can_advance_without_replaying_the_first_page(self):
        ident = self.call('create', 'claude', '--prompt', 'Paged context')['session']['id']
        database = self.root/'state/codejournal/delegations/delegations.sqlite3'
        with closing(sqlite3.connect(database)) as db, db:
            db.executemany('INSERT INTO events(delegation_id,kind,sender,payload,created_at) VALUES(?,?,?,?,?)',
                [(ident, 'message', 'host', json.dumps({'text':str(n)}), 1) for n in range(510)])
        page = self.call('sync', ident, '--full')
        self.assertEqual(len(page['events']), 500)
        self.assertIsNotNone(page['next_after'])
        following = self.call('sync', ident, '--full', '--after', str(page['next_after']))
        self.assertGreater(following['events'][0]['seq'], page['events'][-1]['seq'])
        self.assertIsNone(following['next_after'])

    def test_progress_reminds_the_guest_and_result_is_fenced_until_host_messages_are_consumed(self):
        ident = self.call('create', 'claude', '--prompt', 'Synthetic corrections')['session']['id']
        database = self.root/'state/codejournal/delegations/delegations.sqlite3'
        with closing(sqlite3.connect(database)) as db, db:
            db.execute('UPDATE delegations SET attempt_token=? WHERE id=?', ('synthetic-capability', ident))
        guest = dict(self.env, CJ_DELEGATION_ID=ident, CJ_DELEGATION_ROLE='guest', CJ_DELEGATION_TOKEN='synthetic-capability')
        self.call('publish', ident, '--role', 'guest', '--kind', 'ready', env=guest)
        activity = self.call('status', ident)['session']['last_activity']
        message = self.call('send', ident, '--body', 'Reconsider the corrected input')
        self.assertEqual(message['session']['last_activity'], activity)
        progress = self.call('publish', ident, '--role', 'guest', '--kind', 'status', '--body', 'Working', env=guest)
        self.assertTrue(progress['host_messages']['unread'])
        blocked = self.invoke('publish', ident, '--role', 'guest', '--kind', 'result', '--body', 'Premature verdict', env=guest, check=False)
        self.assertNotEqual(blocked.returncode, 0)
        self.assertIn('unread host messages', blocked.stderr)
        unread = self.call('sync', ident, '--role', 'guest', env=guest)
        self.assertTrue(any(e['payload'].get('text') == 'Reconsider the corrected input' for e in unread['events']))
        self.call('ack', ident, '--role', 'guest', '--through', str(unread['latest_seq']), env=guest)
        self.call('publish', ident, '--role', 'guest', '--kind', 'result', '--body', 'Corrected verdict', env=guest)
        self.assertEqual(self.call('result', ident, '--timeout', '0')['result'], 'Corrected verdict')

    def test_recovery_changes_the_budget_preserves_history_and_rejects_invalid_budgets(self):
        ident = self.start('recover')
        self.settled(ident)
        rejected = self.invoke('recover', ident, '--stall-timeout', 'NaN', check=False)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertEqual(self.call('status', ident)['session']['attempt'], 1)
        self.call('recover', ident[:8], '--stall-timeout', '1800', behavior='recover')
        recovered = self.settled(ident)
        self.assertEqual(recovered['session']['stall_timeout'], 1800)
        event = [e for e in recovered['events'] if e['kind'] == 'attempt_started'][-1]
        self.assertEqual(event['payload']['stall_timeout'], 1800)
        self.assertTrue(event['payload']['previous_error'])
