from contextlib import closing
import os
import sqlite3
import time
from delegation_support import DelegationCase


class DelegationDiagnosticsTest(DelegationCase):
    def test_local_tail_is_bounded_redacted_and_rejects_symlinks(self):
        ident = self.call('create', 'claude', '--prompt', 'Diagnostics')['session']['id']
        directory = self.root/'state/codejournal/delegations'
        token = 'synthetic-private-attempt-capability'
        with closing(sqlite3.connect(directory/'delegations.sqlite3')) as db, db:
            db.execute('UPDATE delegations SET attempt_token=? WHERE id=?', (token, ident))
        path = directory/f'{ident}-attempt-0.stdout.log'
        path.write_text('first\n' + 'line\n'*100 + token+'\nlast\n')
        tail = self.call('output', ident[:8], '--lines', '3')
        self.assertEqual(tail['source'], 'local')
        self.assertNotIn(token, str(tail))
        self.assertIn('[redacted]', tail['streams']['stdout']['text'])
        self.assertLessEqual(len(tail['streams']['stdout']['text'].splitlines()), 3)
        self.assertTrue(tail['streams']['stdout']['tail_truncated'])
        self.assertFalse(tail['streams']['stderr']['available'])
        path.unlink()
        private = self.root/'private.txt'
        private.write_text('not provider output')
        path.symlink_to(private)
        rejected = self.invoke('output', ident, check=False)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertNotIn('not provider output', rejected.stdout)
        self.assertIn('regular file', rejected.stderr)
        path.unlink()
        os.mkfifo(path)
        rejected = self.invoke('output', ident, check=False)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn('regular file', rejected.stderr)

    def test_managed_claude_streams_without_changing_synchronous_output(self):
        dry = self.invoke('run', 'claude', '--prompt', 'Synchronous', '--dry-run')
        self.assertNotIn('--include-partial-messages', dry.stdout)
        ident = self.start('streaming_review', target='claude')
        deadline = time.monotonic()+3
        while time.monotonic() < deadline:
            current = self.call('output', ident)
            if 'Reading the next review input' in current['streams']['stdout']['text']:
                break
            time.sleep(.04)
        self.assertIn('Reading the next review input', current['streams']['stdout']['text'])
        self.settled(ident)

    def test_provider_output_time_is_distinct_from_supervisor_contact(self):
        ident = self.start('streaming_review', target='grok')
        deadline = time.monotonic()+3
        while time.monotonic() < deadline:
            current = self.call('status', ident)
            if current['session']['provider_output_at']:
                break
            time.sleep(.04)
        self.assertIsNotNone(current['session']['provider_output_at'])
        self.assertIn('Reading the next review input', self.call('output', ident)['streams']['stdout']['text'])
        self.settled(ident)

    def test_signal_exit_is_reported_without_inventing_a_killer(self):
        script = self.bin/'claude'
        script.write_text(f'#!{os.sys.executable}\nimport os, signal\nos.kill(os.getpid(), signal.SIGTERM)\n')
        script.chmod(0o755)
        ident = self.start()
        state = self.settled(ident)
        self.assertIn('signal 15', state['session']['error'])
        self.assertIn('unknown', state['session']['error'])
