import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class TaskBodyEditTest(AuditCase):
    def test_body_file_and_base_preserve_the_task_identity(self):
        result = self.cli('--json', '--project', 'fixture', 'task', 'edit', RESOURCE[:8],
                          '--body-file', '-', '--base', 'a' * 64, stdin='Corrected details\n')
        self.assertEqual(result.returncode, 0, result.stderr)
        method, path, body = Handler.calls[-1]
        self.assertEqual((method, path), ('PATCH', '/api/v1/tenants/demo/projects/fixture/tasks/aaaaaaaa'))
        self.assertEqual((body['body'], body['based_on'], body['action']), ('Corrected details', 'a' * 64, 'edit'))
        result = self.cli('--project', 'fixture', 'task', 'edit', RESOURCE, '--body', '', '--base', 'b' * 64)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(Handler.calls[-1][2]['body'], '')

    def test_unprotected_or_oversized_updates_never_send_or_queue(self):
        for args, stdin in [(['--body', 'Unsafe'], None),
                            (['--base', 'a' * 64, '--body-file', '-'], 'é' * 100001)]:
            result = self.cli('--project', 'fixture', 'task', 'edit', RESOURCE, *args, stdin=stdin)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(Handler.calls, [])

    def test_offline_conflicting_updates_retain_their_original_body_and_base(self):
        result = self.cli('--json', '--offline', '--project', 'fixture', 'task', 'edit', RESOURCE,
                          '--body', 'Corrected', '--base', 'a' * 64)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)['queued'])
        path = next((self.base / 'state').rglob('requests/*.json'))
        original = path.read_bytes()
        with patch.object(Handler, 'do_PATCH', lambda handler: handler.reply(409, {'message': 'Task body changed'})):
            failed = self.cli('sync')
        self.assertNotEqual(failed.returncode, 0)
        self.assertEqual(path.read_bytes(), original)
