from contextlib import closing
import json
import sqlite3
import time
from delegation_support import DelegationCase


class DelegationSupervisionTest(DelegationCase):
    def create(self, prompt='Synthetic bounded work'):
        return self.call('create', 'claude', '--prompt', prompt)['session']['id']

    def test_prefixes_resolve_consistently_and_ambiguous_prefixes_never_mutate(self):
        ident = self.create()
        short = ident.replace('-', '')[:8]
        self.assertEqual(self.call('status', short)['session']['id'], ident)
        self.call('send', short, '--body', 'One message')
        state = self.call('sync', short)
        self.call('ack', short, '--through', str(state['latest_seq']))
        self.assertEqual(self.call('output', short)['id'], ident)
        other = ident[:-1] + ('0' if ident[-1] != '0' else '1')
        path = self.root/'state/codejournal/delegations/delegations.sqlite3'
        with closing(sqlite3.connect(path)) as db, db:
            columns = [row[1] for row in db.execute('PRAGMA table_info(delegations)')]
            values = list(db.execute('SELECT * FROM delegations WHERE id=?', (ident,)).fetchone())
            values[columns.index('id')] = other
            db.execute('INSERT INTO delegations VALUES (' + ','.join('?' for _ in values) + ')', values)
        rejected = self.invoke('cancel', short, check=False)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn('ambiguous', rejected.stderr)
        self.assertEqual(self.call('status', ident)['session']['status'], 'created')
        self.call('cancel', ident)
        self.call('cancel', other)

    def test_bare_wait_delivers_old_events_once_without_acknowledging_them(self):
        ident = self.create()
        first = self.call('wait', ident, '--timeout', '.3')
        self.assertFalse(first['timed_out'])
        self.assertTrue(first['events'])
        before = time.monotonic()
        second = self.call('wait', ident, '--timeout', '.3')
        self.assertGreaterEqual(time.monotonic()-before, .25)
        self.assertTrue(second['timed_out'])
        self.assertEqual(second['events'], [])
        self.assertEqual(second['session']['host_ack_seq'], 0)
        self.assertTrue(self.call('sync', ident)['events'])
        self.assertTrue(self.call('wait', ident, '--after', '0', '--timeout', '0')['events'])

    def test_result_and_terminal_wait_do_not_treat_statuses_as_completion(self):
        ident = self.create()
        pending = self.call('result', ident, '--timeout', '.2')
        self.assertTrue(pending['timed_out'])
        self.assertIsNone(pending['result'])
        self.assertEqual(pending['events'], [])
        self.call('cancel', ident, '--reason', 'Synthetic terminal outcome')
        finished = self.call('wait', ident, '--until', 'terminal', '--timeout', '1200')
        self.assertEqual(finished['session']['status'], 'cancelled')
        self.assertFalse(finished['timed_out'])
        ident = self.start()
        self.settled(ident)
        result = self.call('result', ident, '--timeout', '0')
        self.assertEqual(result['result'], 'Standalone provider completed')
        self.assertEqual(result['events'], [])
        self.assertNotIn('result', result['session'])

    def test_compact_responses_omit_the_task_and_full_reads_preserve_it(self):
        prompt = 'Synthetic title\n' + 'Private task detail. '*1000
        ident = self.create(prompt)
        compact = self.call('status', ident)
        self.assertNotIn('task', compact['session'])
        self.assertEqual(compact['id'], ident)
        self.assertLess(len(json.dumps(compact)), 5000)
        self.assertEqual(self.call('status', ident, '--full')['session']['task'], prompt.strip())
        raw = self.invoke('create', 'claude', '--prompt', 'ID only', '--id-only').stdout.strip()
        self.assertEqual(len(raw), 36)
        self.assertEqual(self.call('status', raw)['id'], raw)

    def test_explicit_cwd_filters_the_inventory_before_limits(self):
        one = self.root/'one'
        two = self.root/'two'
        one.mkdir()
        two.mkdir()
        ident = self.call('create', 'claude', '--prompt', 'One', cwd=one)['session']['id']
        self.call('create', 'claude', '--prompt', 'Two', cwd=two)
        scoped = self.call('list', '--active', '--cwd', str(one), '--limit', '1')
        self.assertEqual([s['id'] for s in scoped['sessions']], [ident])
        self.assertEqual(scoped['scope'], 'cwd')
        self.assertEqual(len(self.call('list')['sessions']), 2)

    def test_status_and_list_can_read_while_another_client_holds_the_writer(self):
        ident = self.create()
        path = self.root/'state/codejournal/delegations/delegations.sqlite3'
        with closing(sqlite3.connect(path)) as db:
            db.execute('BEGIN IMMEDIATE')
            started = time.monotonic()
            self.assertEqual(self.call('status', ident)['id'], ident)
            self.assertEqual(len(self.call('list')['sessions']), 1)
            self.assertLess(time.monotonic()-started, 2)
            db.rollback()
