import json
from feedback_audit_support import AuditCase, Handler, RESOURCE


class TaskContextFeedbackTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    def test_priorities_are_discoverable_and_invalid_values_never_reach_the_server(self):
        for action in ['add', 'edit', 'list']:
            args = ['task', action] + ([RESOURCE] if action == 'edit' else [])
            help_text = self.cli(*args, '--help').stdout
            self.assertIn('low, normal, high, critical', help_text)
            failed = self.cli(*args, '--priority', 'urgent')
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn('possible values:', failed.stderr)
        self.assertEqual(Handler.calls, [])

    def test_append_is_a_protected_atomic_operation_and_missing_bases_have_one_line_guidance(self):
        invalid = self.cli('task', 'edit', RESOURCE, '--append-body', 'Extra context')
        self.assertNotEqual(invalid.returncode, 0)
        self.assertIn('cj task show', invalid.stderr.splitlines()[-1])
        self.assertIn('task.body_version', invalid.stderr)
        self.assertEqual(Handler.calls, [])
        result = self.cli('--project', 'fixture', 'task', 'edit', RESOURCE,
                          '--append-body', 'Extra context', '--base', 'a'*64)
        self.assertEqual(result.returncode, 0, result.stderr)
        body = Handler.calls[-1][2]
        self.assertEqual(body['append_body'], 'Extra context')
        self.assertEqual(body['based_on'], 'a'*64)
        self.assertNotIn('body', body)

    def test_refs_can_be_replaced_or_cleared_and_plan_links_use_the_existing_plan_relation(self):
        changed = self.cli('--project', 'fixture', 'task', 'edit', RESOURCE, '--ref', 'https://example.test/source/commit/abc')
        self.assertEqual(changed.returncode, 0, changed.stderr)
        self.assertEqual(Handler.calls[-1][2]['refs'], [{'kind':'url','value':'https://example.test/source/commit/abc'}])
        self.assertEqual(self.cli('--project', 'fixture', 'task', 'edit', RESOURCE, '--clear-refs').returncode, 0)
        self.assertEqual(Handler.calls[-1][2]['refs'], [])
        linked = self.cli('--project', 'fixture', 'task', 'link', RESOURCE, 'plan:bbbbbbbb')
        self.assertEqual(linked.returncode, 0, linked.stderr)
        self.assertEqual(Handler.calls[-1][2]['plan'], 'bbbbbbbb')
        self.assertNotIn('refs', Handler.calls[-1][2])

    def test_cross_project_tasks_reject_unqualified_repository_refs_but_keep_qualified_urls(self):
        for ref in ['commit:abc1234', 'path:src/app.rs', 'branch:main']:
            invalid = self.cli('--project', 'fixture', 'task', 'add', '--to', 'target', '--title', 'Work', '--ref', ref)
            self.assertNotEqual(invalid.returncode, 0)
            self.assertIn('source repository URL', invalid.stderr)
        self.assertEqual(Handler.calls, [])
        valid = self.cli('--project', 'fixture', 'task', 'add', '--to', 'target', '--title', 'Work', '--ref', 'https://example.test/source/commit/abc1234')
        self.assertEqual(valid.returncode, 0, valid.stderr)
        self.assertEqual(Handler.calls[-1][2]['refs'][0]['kind'], 'url')

    def test_entry_show_alias_reuses_the_detail_endpoint(self):
        shown = self.cli('--json', 'entry', 'show', RESOURCE, '--no-track')
        self.assertEqual(shown.returncode, 0, shown.stderr)
        self.assertIn('/entries/'+RESOURCE, Handler.calls[-1][1])
