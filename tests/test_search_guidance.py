import json
import urllib.parse
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class SearchGuidanceTest(AuditCase):
    @staticmethod
    def empty(handler):
        Handler.calls.append(('GET', handler.path, None))
        handler.reply(200, {'entries': [], 'search_mode': 'whole_word'})

    def test_whole_word_mode_is_explicit_and_compatible_with_literal(self):
        with patch.object(Handler, 'do_GET', self.empty):
            for flags in [[], ['--literal']]:
                result = self.cli('--json', 'search', 'REPL', '--whole-word', '--all-projects', *flags)
                self.assertEqual(result.returncode, 0, result.stderr)
                query = urllib.parse.parse_qs(urllib.parse.urlparse(Handler.calls[-1][1]).query)
                self.assertEqual(query['whole_word'], ['1'])
                self.assertEqual(query['q'], ['REPL'])
                self.assertTrue(all('requires all terms' not in note for note in json.loads(result.stdout)['notices']))

    def test_older_server_cannot_silently_ignore_whole_word(self):
        result = self.cli('search', 'REPL', '--whole-word', '--all-projects')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('does not support whole-word search', result.stderr)

    def test_empty_multiterm_search_explains_all_terms_and_any_in_text_and_json(self):
        with patch.object(Handler, 'do_GET', self.empty):
            for flags in [[], ['--json'], ['--offline', '--json']]:
                result = self.cli(*flags, 'search', 'project format database migration assets backup', '--all-projects')
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('requires all terms', result.stdout)
                self.assertIn('--any', result.stdout)
                if '--json' in flags:
                    self.assertEqual(len(json.loads(result.stdout)['notices']), 2)
        self.assertEqual(len(Handler.calls), 2)

    def test_no_irrelevant_hint_for_single_term_filters_literal_any_or_success(self):
        with patch.object(Handler, 'do_GET', self.empty):
            for args in [[], ['repl'], ['a b', '--literal'], ['a b', '--whole-word'], ['a b', '--any'], ['___']]:
                result = self.cli('--json', 'search', '--all-projects', *args)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue(all('requires all terms' not in note for note in json.loads(result.stdout)['notices']))
        with patch.object(Handler, 'do_GET', lambda h: h.reply(200, {'entries': [
                {'id': RESOURCE, 'title': 'A match', 'kind': 'howto', 'status': 'active'}]})):
            self.assertEqual(json.loads(self.cli('--json', 'search', 'a b', '--all-projects').stdout)['notices'], [])

    def test_invalid_whole_word_flags_fail_before_network(self):
        for args in [[], ['word', '--prefix'], ['word', '--any']]:
            self.assertNotEqual(self.cli('search', '--whole-word', *args).returncode, 0)
        self.assertEqual(Handler.calls, [])
