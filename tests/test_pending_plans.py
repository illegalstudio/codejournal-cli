import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler
from pending_plan_server import Plans


class PendingPlansTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.plans = Plans()
        self.enterContext(patch.object(Handler, "do_POST", lambda handler: self.plans.write(handler)))
        self.enterContext(patch.object(Handler, "do_PATCH", lambda handler: self.plans.write(handler)))
        self.enterContext(patch.object(Handler, "do_GET", lambda handler: self.plans.read(handler)))

    def create(self, offline=True):
        result = self.cli("--json", "--project", "fixture", *(["--offline"] if offline else []),
                          "plan", "create", "--title", "Offline work", "--body", "- [ ] First\n- [ ] Second")
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_offline_creation_and_steps_preserve_requests_and_replay_in_order_once(self):
        result = self.create()
        ident = result["resource_id"]
        self.assertNotEqual(ident, result["request_id"])
        creation = next((self.base / "state").rglob("requests/*.json"))
        original = creation.read_bytes()
        for step in [1, 2]:
            updated = self.cli("--offline", "--json", "plan", "step", ident, str(step), "--done")
            self.assertEqual(updated.returncode, 0, updated.stderr)
        projected = self.cli("--offline", "--json", "plan", "show", ident)
        self.assertEqual(projected.returncode, 0, projected.stderr)
        payload = json.loads(projected.stdout)
        self.assertTrue(payload["local_pending"])
        self.assertFalse(payload["synced"])
        self.assertEqual(payload["plan"]["revision"], 3)
        self.assertEqual(payload["plan"]["body"], "- [x] First\n- [x] Second")
        self.assertEqual(creation.read_bytes(), original)
        self.assertEqual(len(list((self.base / "state").rglob("requests/*.json"))), 3)
        sync = self.cli("--json", "sync")
        self.assertEqual(sync.returncode, 0, sync.stderr)
        self.assertEqual(self.plans.items[ident]["revision"], 3)
        self.assertEqual(len(self.plans.items), 1)
        self.assertEqual([body["based_on"] for _, method, body in self.plans.attempts if method == "PATCH"], [1, 2])
        self.assertFalse(list((self.base / "state").rglob("requests/*.json")))
        self.cli("sync")
        self.assertEqual(len(self.plans.attempts), 3)

    def test_uncertain_creation_and_later_update_use_reserved_identity_without_rewriting(self):
        self.plans.uncertain = True
        queued = self.create(offline=False)
        ident = queued["resource_id"]
        path = next((self.base / "state").rglob("requests/*.json"))
        original = path.read_bytes()
        update = self.cli("--offline", "--json", "plan", "update", ident, "--body", "Revised pending content")
        self.assertEqual(update.returncode, 0, update.stderr)
        self.assertEqual(path.read_bytes(), original)
        synchronized = self.cli("sync")
        self.assertEqual(synchronized.returncode, 0, synchronized.stderr)
        self.assertEqual(len(self.plans.items), 1)
        self.assertEqual(self.plans.items[ident]["body"], "Revised pending content")
        self.assertEqual(self.plans.attempts[0][0], self.plans.attempts[1][0])

    def test_conflict_retains_pending_edit_and_does_not_replace_remote_content(self):
        queued = self.create()
        ident = queued["resource_id"]
        self.cli("--offline", "plan", "step", ident, "1", "--done")
        self.plans.conflict = True
        result = self.cli("sync")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.plans.items[ident]["body"], "- [ ] First\n- [ ] Second")
        self.assertEqual(len(list((self.base / "state").rglob("requests/*.json"))), 1)

    def test_other_credentials_cannot_read_pending_plan_or_write_a_step_from_it(self):
        queued = self.create()
        self.env["CJ_TOKEN"] = "different-synthetic-account"
        result = self.cli("--offline", "plan", "step", queued["resource_id"], "1", "--done")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(list((self.base / "state").rglob("requests/*.json"))), 1)

    def test_confirmed_creation_supports_cached_pending_steps_and_rejects_projected_history(self):
        created = self.create(offline=False)
        ident = created['plan']['id']
        queued = self.cli('--offline', 'plan', 'step', ident, '1', '--done')
        self.assertEqual(queued.returncode, 0, queued.stderr)
        projected = self.cli('--offline', '--json', 'plan', 'show', ident)
        self.assertEqual(projected.returncode, 0, projected.stderr)
        data = json.loads(projected.stdout)
        self.assertEqual(data['plan']['revision'], 2)
        self.assertEqual(data['current_revision'], 2)
        history = self.cli('--offline', 'plan', 'show', ident, '--history')
        self.assertNotEqual(history.returncode, 0)
        self.assertIn('synchronize before reading history', history.stderr)
        synchronized = self.cli('sync')
        self.assertEqual(synchronized.returncode, 0, synchronized.stderr)
        self.assertEqual(self.plans.items[ident]['body'], '- [x] First\n- [ ] Second')

    def test_pending_body_cleaning_and_global_document_scope_match_server_semantics(self):
        queued = self.cli('--offline', '--json', '--project', 'fixture', 'plan', 'create',
                          '--title', 'Work', '--body', '  # Work\n\n- [ ] First  ')
        ident = json.loads(queued.stdout)['resource_id']
        shown = self.cli('--offline', '--json', 'plan', 'show', ident)
        self.assertEqual(json.loads(shown.stdout)['plan']['body'], '- [ ] First')
        doc = self.cli('--offline', '--json', 'doc', 'create', '--global', '--title', 'Shared', '--body', 'Text')
        self.assertEqual(doc.returncode, 0, doc.stderr)
        shown_doc = self.cli('--offline', '--json', 'doc', 'show', json.loads(doc.stdout)['resource_id'])
        self.assertEqual(shown_doc.returncode, 0, shown_doc.stderr)
        data = json.loads(shown_doc.stdout)
        self.assertEqual(data['doc']['scope'], 'global')
        self.assertIsNone(data['doc']['project_slug'])
