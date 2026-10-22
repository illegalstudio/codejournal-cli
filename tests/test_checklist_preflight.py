import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class ChecklistPreflightTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    @staticmethod
    def detail(handler):
        Handler.calls.append(('GET', handler.path, None))
        handler.reply(200, {'plan': {'id': RESOURCE, 'title': 'Plan', 'revision': 3,
                                    'body': '1. [ ] First\n```md\n- [ ] Example\n```\n2) [ ] Second'}})

    def test_failed_online_read_does_not_mutate_using_cached_revision(self):
        with patch.object(Handler, 'do_GET', self.detail):
            self.assertEqual(self.cli('plan', 'show', RESOURCE).returncode, 0)
        Handler.calls.clear()
        with patch.object(Handler, 'do_GET', lambda h: h.reply(503, {'message': 'Unavailable'})):
            failed = self.cli('plan', 'step', RESOURCE, '1', '--done')
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn('fresh checklist revision', failed.stderr)
        self.assertIn('nothing was sent or queued', failed.stderr)
        self.assertFalse(any(method == 'PATCH' for method, _, _ in Handler.calls))
        self.assertFalse(list((self.base / 'state').rglob('requests/*.json')))
        offline = self.cli('--offline', 'plan', 'step', RESOURCE, '2', '--done')
        self.assertEqual(offline.returncode, 0, offline.stderr)
        queued = json.loads(next((self.base / 'state').rglob('requests/*.json')).read_text())
        self.assertEqual(queued['body']['based_on'], 3)

    def test_plain_lists_and_invalid_indexes_are_rejected_before_any_write(self):
        for body, index, count in [('1. First\n2. Second', '1', 0),
                                   ('```md\n- [ ] Example\n```', '1', 0),
                                   ('- [ ] First', '2', 1)]:
            with self.subTest(body=body), patch.object(Handler, 'do_GET', lambda h: h.reply(
                    200, {'plan': {'id': RESOURCE, 'revision': 1, 'body': body}})):
                failed = self.cli('plan', 'step', RESOURCE, index, '--done')
                self.assertNotEqual(failed.returncode, 0)
                self.assertIn(f'detected {count} items', failed.stderr)
                self.assertIn('1. [ ]', failed.stderr)
        self.assertFalse(any(method == 'PATCH' for method, _, _ in Handler.calls))
        self.assertFalse(list((self.base / 'state').rglob('requests/*.json')))

    def test_valid_numbered_checklist_sends_only_the_atomic_change(self):
        with patch.object(Handler, 'do_GET', self.detail):
            changed = self.cli('plan', 'step', RESOURCE, '2', '--done')
        self.assertEqual(changed.returncode, 0, changed.stderr)
        body = next(body for method, _, body in Handler.calls if method == 'PATCH')
        self.assertEqual((body['based_on'], body['step_index'], body['step_done']), (3, 2, True))
        self.assertNotIn('body', body)
