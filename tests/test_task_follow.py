import json
from unittest.mock import patch

from feedback_audit_support import AuditCase, Handler, RESOURCE


class TaskFollowTest(AuditCase):
    def test_follow_and_unfollow_use_the_selected_consumer_project(self):
        def change(handler):
            handler.rfile.read(int(handler.headers.get('Content-Length', 0)))
            Handler.calls.append((handler.command, handler.path, None))
            handler.reply(200, {'task_id': RESOURCE, 'project': 'consumer',
                                'following': handler.command == 'PUT', 'changed': True})

        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'
        with patch.object(Handler, 'do_PUT', change, create=True), patch.object(Handler, 'do_DELETE', change, create=True):
            for verb, method, following in [('follow', 'PUT', True), ('unfollow', 'DELETE', False)]:
                result = self.cli('--json', 'task', verb, RESOURCE, '--project', 'consumer')
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIs(json.loads(result.stdout)['following'], following)
                self.assertEqual(Handler.calls[-1][:2], (method, f'/api/v1/tenants/demo/tasks/{RESOURCE}/consumers/consumer'))
