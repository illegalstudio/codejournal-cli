import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler


class ProjectArchiveTest(AuditCase):
    @staticmethod
    def lifecycle(handler):
        body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
        Handler.calls.append(('POST', handler.path, body))
        handler.reply(200, {'project': {'slug': 'fixture', 'name': 'Fixture',
                                      'rules': 'PRIVATE CONTEXT',
                                      'archived_at': '2026-10-01' if body['archived'] else None}})

    def test_archive_restore_are_online_and_never_bootstrap_or_queue(self):
        with patch.object(Handler, 'do_POST', self.lifecycle):
            archived = self.cli('--project', 'fixture', 'project', 'archive')
            self.assertEqual(archived.returncode, 0, archived.stderr)
            self.assertIn('Archived project fixture', archived.stdout)
            self.assertEqual(len(Handler.calls), 1)
            self.assertTrue(Handler.calls[0][1].endswith('/projects/fixture/archive'))
            offline = self.cli('--offline', '--project', 'fixture', '--json', 'brief')
            self.assertEqual(offline.returncode, 0, offline.stderr)
            self.assertTrue(json.loads(offline.stdout)['archived'])
            self.assertNotIn('PRIVATE CONTEXT', offline.stdout)
            restored = self.cli('--project', 'fixture', 'project', 'restore')
            self.assertEqual(restored.returncode, 0, restored.stderr)
            self.assertIn('Restored project fixture', restored.stdout)
        calls = len(Handler.calls)
        for action in ['archive', 'restore']:
            result = self.cli('--offline', '--project', 'fixture', 'project', action)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('require an online connection', result.stderr)
        self.assertEqual(len(Handler.calls), calls)

    def test_archived_brief_replaces_active_caches_and_suppresses_context(self):
        active = self.cli('--project', 'fixture', 'brief')
        self.assertIn('Preserve known rules', active.stdout)

        def archived(handler):
            handler.rfile.read(int(handler.headers['Content-Length']))
            handler.reply(200, {'archived': True, 'project': {'slug': 'fixture'},
                                'code': 'project_archived'})

        with patch.object(Handler, 'do_POST', archived):
            result = self.cli('--project', 'fixture', 'brief')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Archived: read-only', result.stdout)
        self.assertNotIn('rules are empty', result.stdout)
        cached = self.cli('--offline', '--project', 'fixture', 'brief', '--all', '--verbose')
        self.assertEqual(cached.returncode, 0, cached.stderr)
        self.assertIn('Archived: read-only', cached.stdout)
        self.assertNotIn('Preserve known rules', cached.stdout)
        with patch.object(Handler, 'do_POST', self.lifecycle):
            self.assertEqual(self.cli('--project', 'fixture', 'project', 'restore').returncode, 0)
        restored = self.cli('--offline', '--project', 'fixture', 'brief')
        self.assertNotEqual(restored.returncode, 0)
        self.assertIn('Run cj brief online', restored.stderr)

    def test_project_listing_has_active_archived_and_all_filters(self):
        def listing(handler):
            Handler.calls.append(('GET', handler.path, None))
            handler.reply(200, {'projects': [{'slug': 'active', 'name': 'Active'},
                                            {'slug': 'archived', 'name': 'Archived', 'archived_at': '2026-10-01'}]})

        with patch.object(Handler, 'do_GET', listing):
            for flags, expected in [((), ['active']), (('--archived',), ['archived']),
                                     (('--all',), ['active', 'archived'])]:
                result = self.cli('--json', 'projects', *flags)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual([item['slug'] for item in json.loads(result.stdout)['projects']], expected)
            conflict = self.cli('projects', '--archived', '--all')
            self.assertNotEqual(conflict.returncode, 0)

    def test_archived_hook_does_not_record_activity_or_prompt_for_new_rules(self):
        def archived(handler):
            handler.rfile.read(int(handler.headers['Content-Length']))
            handler.reply(200, {'archived': True, 'project': {'slug': 'fixture'}})

        self.env['CJ_PROJECT'] = 'fixture'
        payload = json.dumps({'session_id': 'archived-hook', 'cwd': str(self.repo), 'source': 'compact'})
        with patch.object(Handler, 'do_POST', archived):
            result = self.cli('hook', 'SessionStart', stdin=payload)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Archived: read-only', result.stdout)
        self.assertFalse(json.loads(result.stdout)['journal_active'])
        self.assertNotIn('record with `cj add`', result.stdout)
        queued = list((self.base / 'state').rglob('*.jsonl'))
        self.assertEqual(queued, [])
