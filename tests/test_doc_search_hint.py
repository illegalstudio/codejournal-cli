import json
import shlex
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class DocSearchHintTest(AuditCase):
    @staticmethod
    def empty(handler):
        Handler.calls.append(('GET', handler.path, None))
        handler.reply(200, {'entries': [], 'project': {'slug': 'fixture'}})

    def test_local_empty_query_suggests_current_local_docs_in_text_json_and_cache(self):
        with patch.object(Handler, 'do_GET', self.empty):
            for flags in [[], ['--json'], ['--offline', '--json']]:
                result = self.cli(*flags, '--project', 'fixture', 'search', 'agent')
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('cj doc list --local --project fixture --grep agent', result.stdout)
                if '--json' in flags:
                    self.assertEqual(len(json.loads(result.stdout)['notices']), 1)
        self.assertEqual(len(Handler.calls), 2)

    def test_cross_project_hints_keep_scope_and_quote_arbitrary_queries_as_one_argument(self):
        query = "worker's $(command); notes"
        with patch.object(Handler, 'do_GET', self.empty):
            result = self.cli('--json', 'search', query, '--all-projects', '--literal')
        self.assertEqual(result.returncode, 0, result.stderr)
        notice = json.loads(result.stdout)['notices'][0]
        command = notice.split('with: ', 1)[1]
        self.assertEqual(shlex.split(command), ['cj', 'doc', 'list', '--all-projects', '--grep', query])
        self.assertEqual(len(Handler.calls), 1)

    def test_recent_filter_only_and_successful_searches_have_no_doc_hint(self):
        with patch.object(Handler, 'do_GET', self.empty):
            for args in [('search', '--kind', 'gotcha'), ('recent',)]:
                value = json.loads(self.cli('--json', '--project', 'fixture', *args).stdout)
                self.assertEqual(value['notices'], [])
        with patch.object(Handler, 'do_GET', lambda h: h.reply(200, {'entries': [
                {'id': RESOURCE, 'title': 'Match', 'status': 'active', 'kind': 'howto'}]})):
            value = json.loads(self.cli('--json', 'search', 'agent', '--all-projects').stdout)
            self.assertEqual(value['notices'], [])
