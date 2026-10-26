import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class PlanSizeTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    @staticmethod
    def detail(handler):
        Handler.calls.append(('GET', handler.path, None))
        noun = 'doc' if '/docs/' in handler.path else 'plan'
        historical = 'revision=1' in handler.path
        handler.reply(200, {noun: {'id': RESOURCE, 'title': 'Synthetic', 'body': 'é' * 90000,
                                 'status': 'current' if noun == 'doc' else 'active', 'revision': 1 if historical else 2},
                           'current_revision': 2, 'revision_count': 2, 'revisions': []})

    def test_unicode_sizes_and_current_warnings_are_visible_before_content_and_in_json(self):
        with patch.object(Handler, 'do_GET', self.detail):
            for noun in ['plan', 'doc']:
                text = self.cli(noun, 'show', RESOURCE)
                self.assertEqual(text.returncode, 0, text.stderr)
                self.assertTrue(text.stdout.startswith('Body size: 90000/100000'))
                self.assertLess(text.stdout.index('Compact the current state'), text.stdout.index('é' * 10))
                value = json.loads(self.cli('--json', noun, 'show', RESOURCE).stdout)
                self.assertEqual(value['body_size'], {'characters': 90000, 'limit': 100000, 'remaining': 10000, 'warning_at': 90000})
                self.assertEqual(len(value['warnings']), 1)
                self.assertIn('--history', value['warnings'][0])
            self.assertEqual(self.cli('plan', 'show', RESOURCE, '--body').stdout, 'é' * 90000 + '\n')

    def test_historical_reads_show_size_without_suggesting_editing_the_old_revision(self):
        with patch.object(Handler, 'do_GET', self.detail):
            value = json.loads(self.cli('--json', 'plan', 'show', RESOURCE, '--revision', '1').stdout)
        self.assertEqual(value['body_size']['characters'], 90000)
        self.assertEqual(value.get('warnings', []), [])

    def test_cached_and_pending_content_recompute_sizes(self):
        with patch.object(Handler, 'do_GET', self.detail):
            self.assertEqual(self.cli('plan', 'show', RESOURCE).returncode, 0)
        cached = json.loads(self.cli('--json', '--offline', 'plan', 'show', RESOURCE).stdout)
        self.assertEqual(cached['body_size']['characters'], 90000)
        changed = self.cli('--offline', 'plan', 'update', RESOURCE, '--body-file', '-', stdin='Short state')
        self.assertEqual(changed.returncode, 0, changed.stderr)
        result = self.cli('--json', '--offline', 'plan', 'show', RESOURCE)
        self.assertEqual(result.returncode, 0, result.stderr)
        pending = json.loads(result.stdout)
        self.assertTrue(pending['local_pending'])
        self.assertEqual(pending['body_size']['characters'], 11)
        self.assertEqual(pending.get('warnings', []), [])
        updated = self.cli('--offline', 'plan', 'update', RESOURCE, '--body-file', '-', stdin='x' * 99000)
        self.assertEqual(updated.returncode, 0, updated.stderr)
        large = json.loads(self.cli('--json', '--offline', 'plan', 'show', RESOURCE).stdout)
        self.assertEqual(large['body_size']['characters'], 99000)
        self.assertEqual(len(large['warnings']), 1)

    def test_queued_creation_and_mutation_confirmations_include_sizes(self):
        result = self.cli('--json', '--offline', 'plan', 'create', '--title', 'Synthetic', '--body-file', '-', stdin='x' * 99797)
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertTrue(value['queued'])
        self.assertEqual(value['body_size']['remaining'], 203)
        self.assertEqual(len(value['warnings']), 1)
