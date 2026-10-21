import json
import urllib.parse
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class DocSectionsTest(AuditCase):
    def test_section_read_sends_bounded_parameters_and_keeps_truncation_metadata(self):
        def read(handler):
            Handler.calls.append(('GET', handler.path, None))
            handler.reply(200, {'doc': {'id': RESOURCE, 'title': 'Evidence', 'body': '## Current\nReady', 'revision': 7},
                                'current_revision': 7, 'revision_count': 7,
                                'body_selection': {'section': 'Current', 'start_line': 5, 'end_line': 6,
                                                   'full_characters': 20000, 'truncated': True}, 'revisions': []})
        with patch.object(Handler, 'do_GET', read):
            result = self.cli('--json', 'doc', 'show', RESOURCE, '--section', 'Current', '--max-lines', '2')
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(value['doc']['body'], '## Current\nReady')
        self.assertTrue(value['body_selection']['truncated'])
        self.assertEqual(value['body_size']['characters'], 20000)
        query = urllib.parse.parse_qs(urllib.parse.urlparse(Handler.calls[-1][1]).query)
        self.assertEqual(query, {'history': ['0'], 'section': ['Current'], 'max_lines': ['2']})

    def test_unsupported_server_cannot_silently_return_the_full_document(self):
        result = self.cli('doc', 'show', RESOURCE, '--max-lines', '10')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('does not support bounded section reads', result.stderr)

    def test_history_conflicts_and_invalid_bounds_fail_before_authentication(self):
        self.config.unlink()
        for args in [('--section', 'Current', '--history'), ('--max-lines', '501'),
                     ('--max-lines', '0'), ('--section', 'Current', '--all-revisions')]:
            result = self.cli('doc', 'show', RESOURCE, *args)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(Handler.calls, [])
