import json
import urllib.parse
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class LiteralSearchTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    @staticmethod
    def search_response(handler):
        Handler.calls.append(('GET', handler.path, None))
        handler.reply(200, {'entries': [{'id': RESOURCE, 'title': 'Worker health',
            'status': 'active', 'kind': 'howto', 'body': 'FULL BODY',
            'match_excerpt': 'Inspect the liveness handshake.'}]})

    def test_literal_query_is_encoded_and_excerpts_survive_text_and_json_summaries(self):
        with patch.object(Handler, 'do_GET', self.search_response):
            for json_mode in [[], ['--json']]:
                result = self.cli(*json_mode, 'search', '100%_ready worker::probe', '--literal', '--all-projects')
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('liveness handshake', result.stdout)
                self.assertNotIn('FULL BODY', result.stdout)
                query = urllib.parse.parse_qs(urllib.parse.urlparse(Handler.calls[-1][1]).query)
                self.assertEqual(query['q'], ['100%_ready worker::probe'])
                self.assertEqual(query['literal'], ['1'])
                self.assertEqual(query['excerpt'], ['1'])
                self.assertEqual(query['summary'], ['1'])
            self.assertIn('match_excerpt', json.loads(result.stdout)['entries'][0])

    def test_full_text_search_requests_excerpts_without_enabling_literal_matching(self):
        with patch.object(Handler, 'do_GET', self.search_response):
            result = self.cli('search', 'liveness', '--all-projects', '--prefix')
        self.assertEqual(result.returncode, 0, result.stderr)
        query = urllib.parse.parse_qs(urllib.parse.urlparse(Handler.calls[-1][1]).query)
        self.assertEqual(query['excerpt'], ['1'])
        self.assertEqual(query['prefix'], ['1'])
        self.assertNotIn('literal', query)

    def test_conflicting_literal_modes_fail_without_requests(self):
        for args in [('search', '--literal'), ('search', 'word', '--literal', '--prefix'),
                     ('search', 'word', '--literal', '--any')]:
            result = self.cli(*args)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(Handler.calls, [])
