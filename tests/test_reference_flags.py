import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


class ReferenceFlagsTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'

    @staticmethod
    def patched(extra):
        def handler(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            Handler.calls.append(("PATCH", self.path, body))
            self.reply(200, {"doc": {"id": RESOURCE, "revision": 3}, "log": {"id": RESOURCE, "revision": 2, **extra}, **extra})
        return handler

    def body(self):
        return Handler.calls[-1][2]

    def test_add_ref_extends_a_doc_and_the_confirmation_counts_references(self):
        with patch.object(Handler, 'do_PATCH', self.patched({"refs_before": 10, "refs_after": 12})):
            added = self.cli('doc', 'update', 'aaaaaaaa', '--add-ref', 'path:src/a.rs', '--add-ref', 'path:src/b.rs')
            self.assertEqual(added.returncode, 0, added.stderr)
            self.assertEqual(self.body()['add_refs'], [{"kind": "path", "value": "src/a.rs"}, {"kind": "path", "value": "src/b.rs"}])
            self.assertNotIn('refs', self.body())
            self.assertIn('Refs: 10 before, 12 now.', added.stdout)
        with patch.object(Handler, 'do_PATCH', self.patched({"refs_before": 10, "refs_after": 2})):
            replaced = self.cli('doc', 'update', 'aaaaaaaa', '--ref', 'path:src/a.rs', '--ref', 'path:src/b.rs')
            self.assertIn('replaced 10 with 2', replaced.stdout)
            self.assertIn('use --add-ref to keep the existing references', replaced.stdout)
        both = self.cli('doc', 'update', 'aaaaaaaa', '--ref', 'path:a', '--add-ref', 'path:b')
        self.assertNotEqual(both.returncode, 0)
        self.assertIn('cannot be used with', both.stderr)

    def test_an_older_server_cannot_silently_ignore_add_ref(self):
        calls = len(Handler.calls)
        for command in (['doc', 'update', 'aaaaaaaa'], ['log', 'update', 'aaaaaaaa']):
            ignored = self.cli(*command, '--add-ref', 'path:src/a.rs')
            self.assertNotEqual(ignored.returncode, 0)
            self.assertIn('does not support --add-ref', ignored.stderr)
        self.assertGreater(len(Handler.calls), calls)

    def test_log_update_extends_references(self):
        with patch.object(Handler, 'do_PATCH', self.patched({"refs_before": 1, "refs_after": 2})):
            result = self.cli('log', 'update', 'aaaaaaaa', '--add-ref', 'commit:abcdef12')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.body(), {"add_refs": [{"kind": "commit", "value": "abcdef12"}]})
        self.assertIn('Refs: 1 before, 2 now.', result.stdout)

    def test_docs_declare_covered_topics_and_plans_cannot(self):
        with patch.object(Handler, 'do_PATCH', self.patched({})):
            self.assertEqual(self.cli('doc', 'update', 'aaaaaaaa', '--topics', 'strict-mode,eir').returncode, 0)
            self.assertEqual(self.body()['topics'], ['strict-mode', 'eir'])
            self.assertEqual(self.cli('doc', 'update', 'aaaaaaaa', '--clear-topics').returncode, 0)
            self.assertEqual(self.body()['topics'], [])
        calls = len(Handler.calls)
        for command in (['plan', 'update', 'aaaaaaaa', '--topics', 'eir'], ['plan', 'create', '--title', 'T', '--body', 'B', '--topics', 'eir']):
            refused = self.cli(*command)
            self.assertNotEqual(refused.returncode, 0)
            self.assertIn('only supported for docs', refused.stderr)
        self.assertEqual(len(Handler.calls), calls)
        created = self.cli('doc', 'create', '--title', 'Guide', '--body', 'Body', '--topics', 'eir')
        self.assertEqual(created.returncode, 0, created.stderr)
        self.assertEqual(self.body()['topics'], ['eir'])

    def test_add_supersedes_in_one_request_and_warns_when_the_server_did_not(self):
        def created(extra):
            def handler(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                Handler.calls.append(("POST", self.path, body))
                self.reply(201, {"entry": {"id": RESOURCE}, **extra})
            return handler
        old = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'
        with patch.object(Handler, 'do_POST', created({"superseded": old})):
            done = self.cli('add', '--kind', 'gotcha', '--title', 'Fact', '--body', 'New', '--supersedes', 'bbbbbbbb')
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.body()['supersedes'], 'bbbbbbbb')
        self.assertIn('Superseded bbbbbbbb.', done.stdout)
        with patch.object(Handler, 'do_POST', created({})):
            ignored = self.cli('add', '--kind', 'gotcha', '--title', 'Fact', '--body', 'New', '--supersedes', 'bbbbbbbb')
        self.assertIn('cj supersede bbbbbbbb --by aaaaaaaa', ignored.stdout)

    def test_temporary_kinds_and_positional_text_fail_before_any_request(self):
        refused = self.cli('add', '--kind', 'decision', '--title', 'Choice', '--body', 'Why', '--temporary')
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn('discovery, gotcha, howto', refused.stderr)
        for command in (['log', 'add', '--title', 'Done', 'What changed'], ['feedback', 'add', 'It failed']):
            positional = self.cli(*command)
            self.assertEqual(positional.returncode, 2)
            self.assertIn('hint: this command takes no positional text; pass it with --body', positional.stderr)
        self.assertEqual([call for call in Handler.calls if call[0] != 'GET'], [])
        for name in ('supersede', 'obsolete'):
            line = next(row for row in self.cli('--help').stdout.splitlines() if row.strip().startswith(name))
            self.assertGreater(len(line.split()), 3, line)
