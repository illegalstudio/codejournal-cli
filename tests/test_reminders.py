import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE

REMINDER = dict(id=RESOURCE, project_slug='ariadne', title='Review the cache choice',
                body='Wait for practical feedback.', due_at='2030-11-28T08:00:00+00:00',
                timezone='Europe/Rome', status='open', revision=3, delivery_scope='workspace')
TOKEN = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'


def reminder_get(handler):
    handler.calls.append(('GET', handler.path, None))
    if '/reminders?' in handler.path:
        return handler.reply(200, dict(items=[REMINDER], total=1, page=1, per_page=50))
    handler.reply(200, dict(reminder=REMINDER))


def reminder_post(handler):
    body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
    handler.calls.append(('POST', handler.path, body))
    if handler.path.endswith('/brief'):
        return handler.reply(200, dict(project=dict(slug='elephc'), rules='Preserve rules',
                                      reminders=dict(items=[REMINDER], total=1)))
    handler.reply(200, dict(claimed=True, delivery_token=TOKEN, claim_until='2030-11-28T08:10:00Z', reminder=REMINDER))


class ReminderTest(AuditCase):
    def test_add_preserves_context_scope_and_an_explicit_timezone(self):
        result = self.cli('--json', '--project', 'ariadne', 'reminder', 'add',
                          '--title', 'Review the cache choice', '--at', '2030-11-28T09:00',
                          '--timezone', 'Europe/Rome', '--body-file', '-', stdin='Wait for feedback.\n')
        self.assertEqual(result.returncode, 0, result.stderr)
        method, path, body = Handler.calls[-1]
        self.assertEqual((method, path), ('POST', '/api/v1/tenants/demo/reminders'))
        self.assertEqual((body['project'], body['body'], body['timezone'], body['delivery_scope']),
                         ('ariadne', 'Wait for feedback.', 'Europe/Rome', 'workspace'))
        result = self.cli('--json', 'reminder', 'add', '--global', '--title', 'Review priorities',
                          '--at', '2030-11-28', '--timezone', 'UTC')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(Handler.calls[-1][2]['project'])

    def test_discovery_defaults_to_the_workspace_and_omits_bodies(self):
        with patch.object(Handler, 'do_GET', reminder_get):
            result = self.cli('--json', '--project', 'elephc', 'reminder', 'list', '--state', 'due')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn('body', json.loads(result.stdout)['items'][0])
            self.assertNotIn('project=', Handler.calls[-1][1])
            result = self.cli('--json', '--project', 'ariadne', 'reminder', 'list', '--project-only', '--verbose')
            self.assertIn('project=ariadne', Handler.calls[-1][1])
            self.assertEqual(json.loads(result.stdout)['items'][0]['body'], REMINDER['body'])

    def test_updates_use_a_fresh_revision_and_preserve_it_when_queued(self):
        with patch.object(Handler, 'do_GET', reminder_get):
            result = self.cli('--json', 'reminder', 'reschedule', RESOURCE[:8], '--at', '2030-12-12', '--timezone', 'UTC')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(Handler.calls[-1], ('PATCH', '/api/v1/tenants/demo/reminders/aaaaaaaa',
                                               dict(action='reschedule', at='2030-12-12', timezone='UTC', base=3)))
            self.cli('--json', 'reminder', 'show', RESOURCE)
        result = self.cli('--json', '--offline', 'reminder', 'close', RESOURCE)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)['queued'])
        queued = json.loads(next((self.base / 'state').rglob('requests/*.json')).read_text())
        self.assertEqual(queued['body'], dict(action='close', base=3))

    def test_announcements_never_queue_and_include_the_current_project_for_claiming(self):
        with patch.object(Handler, 'do_POST', reminder_post):
            result = self.cli('--json', '--project', 'elephc', 'reminder', 'claim', RESOURCE)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)['delivery_token'], TOKEN)
            self.assertEqual(Handler.calls[-1][2], dict(project='elephc'))
            result = self.cli('reminder', 'presented', RESOURCE, '--delivery-token', TOKEN)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('remains open', result.stdout)
        count = len(Handler.calls)
        for action in [('claim', RESOURCE), ('presented', RESOURCE, '--delivery-token', TOKEN)]:
            result = self.cli('--offline', 'reminder', *action)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('nothing was queued', result.stderr)
        self.assertEqual(len(Handler.calls), count)
        self.assertEqual(list((self.base / 'state').rglob('requests/*.json')), [])

    def test_full_and_compact_briefs_carry_cross_project_reminders_and_the_delivery_protocol(self):
        with patch.object(Handler, 'do_POST', reminder_post):
            for flags in [(), ('--compact',)]:
                result = self.cli('--project', 'elephc', 'brief', *flags)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('[ariadne] Review the cache choice', result.stdout)
                self.assertIn('cj reminder claim ID --json', result.stdout)
                self.assertIn('never closes', result.stdout)
