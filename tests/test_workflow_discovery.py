import urllib.parse
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler


class WorkflowDiscoveryTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    def test_refs_move_accepts_prefixed_and_bare_paths_and_rejects_traversal(self):
        for old, new in [('path:src/old', 'path:src/new'), ('src/old/', 'src/new/')]:
            changed = self.cli('--project', 'fixture', 'refs', 'move', old, new)
            self.assertEqual(changed.returncode, 0, changed.stderr)
            self.assertEqual(Handler.calls[-1][2], {'old': 'src/old', 'new': 'src/new'})
        Handler.calls.clear()
        for old, new in [('path:src/old', 'src/old'), ('path:../outside', 'src/new'), ('path:', 'src/new')]:
            self.assertNotEqual(self.cli('--project', 'fixture', 'refs', 'move', old, new).returncode, 0)
        self.assertEqual(Handler.calls, [])

    def test_task_filter_is_encoded_for_project_and_tenant_discovery(self):
        for scope in [[], ['--all-projects']]:
            query = 'Release & 100%_ready'
            shown = self.cli('--project', 'fixture', 'task', 'list', *scope, '--grep', query)
            self.assertEqual(shown.returncode, 0, shown.stderr)
            url = urllib.parse.urlparse(Handler.calls[-1][1])
            params = urllib.parse.parse_qs(url.query)
            self.assertEqual(params['grep'], [query])
            self.assertEqual(params['status'], ['active'])
            self.assertEqual(params['summary'], ['1'])
            self.assertEqual('/projects/fixture/' in url.path, not scope)

    def test_checkout_removal_preserves_exact_symlink_and_remote_host_paths(self):
        target = self.base / 'new-checkout'
        target.mkdir()
        alias = self.base / 'old-checkout'
        alias.symlink_to(target, target_is_directory=True)
        for command in [('checkouts', 'remove'), ('project', 'path-remove')]:
            removed = self.cli('--project', 'fixture', *command, str(alias), '--host', 'old-host')
            self.assertEqual(removed.returncode, 0, removed.stderr)
            self.assertEqual(Handler.calls[-1][1], '/api/v1/tenants/demo/projects/fixture/paths/remove')
            self.assertEqual(Handler.calls[-1][2], {'host': 'old-host', 'path': str(alias)})
            self.assertTrue(alias.is_symlink())
            self.assertTrue(target.is_dir())
        missing = self.cli('--project', 'fixture', 'checkouts', 'remove', '/absent/checkout', '--host', 'remote')
        self.assertEqual(missing.returncode, 0, missing.stderr)
        self.assertEqual(Handler.calls[-1][2], {'host': 'remote', 'path': '/absent/checkout'})
        Handler.calls.clear()
        failed = self.cli('--project', 'fixture', 'checkouts', 'remove', 'relative/path')
        self.assertNotEqual(failed.returncode, 0)
        self.assertEqual(Handler.calls, [])

    def test_checkout_listing_remains_compatible(self):
        with patch.object(Handler, 'do_GET', lambda h: (Handler.calls.append(('GET', h.path, None)), h.reply(200, {'checkouts': []}))):
            self.assertEqual(self.cli('--project', 'fixture', 'checkouts').returncode, 0)
            self.assertIn('project=fixture', Handler.calls[-1][1])
            self.assertEqual(self.cli('checkouts', '--all-projects').returncode, 0)
            self.assertTrue(Handler.calls[-1][1].endswith('/checkouts'))

    def test_invalid_enum_errors_include_the_accepted_values_without_writes(self):
        for command in [('add', '--kind', 'finding'), ('feedback', 'add', '--category', 'cli')]:
            result = self.cli(*command, '--title', 'Synthetic', '--body', 'Synthetic')
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('possible values:', result.stderr)
        self.assertEqual(Handler.calls, [])
