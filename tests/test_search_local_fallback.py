import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE

OTHER = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'


class SearchLocalFallbackTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    @staticmethod
    def entries(handler):
        Handler.calls.append(('GET', handler.path, None))
        if Handler.failed:
            return handler.reply(500, {'message': 'Server failure'})
        handler.reply(200, {'entries': [
            {'id': RESOURCE, 'title': 'Worker health', 'status': 'active', 'kind': 'howto',
             'created_at': '2026-11-01T00:00:00Z', 'topics': ['workers'], 'match_excerpt': 'Inspect the liveness handshake.'},
            {'id': OTHER, 'title': 'Retired probe', 'status': 'superseded', 'kind': 'gotcha',
             'created_at': '2026-10-01T00:00:00Z', 'topics': [], 'match_excerpt': 'The liveness probe moved.'}]})

    def test_a_failed_search_answers_from_entries_seen_earlier_and_says_so(self):
        with patch.object(Handler, 'do_GET', self.entries):
            online = self.cli('--json', 'search', 'liveness', '--all-projects')
            self.assertEqual(online.returncode, 0, online.stderr)
            self.assertEqual(json.loads(online.stdout)['local_index'], None)
            Handler.failed = True
            same = json.loads(self.cli('--json', 'search', 'liveness', '--all-projects').stdout)
            self.assertTrue(same['cached'])
            self.assertRegex(same['cached_at'], r'^20\d\d-')
            self.assertIn('the same search returned at 20', same['notices'][0])
            other = self.cli('--json', 'search', 'handshake worker', '--all-projects')
            self.assertEqual(other.returncode, 0, other.stderr)
            data = json.loads(other.stdout)
            self.assertEqual([entry['id'] for entry in data['entries']], [RESOURCE])
            self.assertEqual(data['local_index']['indexed'], 2)
            self.assertRegex(data['local_index']['newest_seen_at'], r'^20\d\d-')
            self.assertIn('Entries never seen here are missing', data['notices'][0])
            self.assertNotIn('match_excerpt', data['entries'][0])
            text = self.cli('search', 'liveness', '--all-projects', '--all-statuses', '--kind', 'gotcha')
            self.assertIn('Retired probe', text.stdout)
            self.assertIn('retry online for a complete search', text.stdout)
            self.assertEqual(json.loads(self.cli('--json', 'search', 'nothing-like-it', '--all-projects').stdout)['entries'], [])
            unsupported = self.cli('search', 'liveness', '--all-projects', '--path', 'src/worker.rs')
            self.assertNotEqual(unsupported.returncode, 0)
            self.assertIn('No cached response', unsupported.stderr)

    def test_without_remembered_entries_the_failure_stays_explicit(self):
        Handler.failed = True
        with patch.object(Handler, 'do_GET', self.entries):
            result = self.cli('search', 'liveness', '--all-projects')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('No cached response', result.stderr)
