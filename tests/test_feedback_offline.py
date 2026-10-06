import json
from feedback_audit_support import AuditCase, Handler, RESOURCE


class FeedbackOfflineTest(AuditCase):
    def test_server_errors_report_cache_state_without_os_errors(self):
        Handler.failed = True
        result = self.cli("search", "uncached", "--project", "fixture")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("500", result.stderr)
        self.assertIn("No cached response", result.stderr)
        self.assertIn("cj outbox list", result.stderr)
        self.assertNotIn("No such file", result.stderr)

    def test_cached_brief_retains_rules_and_explains_document_availability(self):
        self.assertEqual(self.cli("brief", "--project", "fixture").returncode, 0)
        Handler.failed = True
        result = self.cli("brief", "--project", "fixture")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Preserve known rules", result.stdout)
        self.assertIn("listed documents may not have cached bodies", result.stdout)
        result = self.cli("doc", "show", RESOURCE, "--current-only")
        self.assertIn("No cached response", result.stderr)

    def test_cached_search_is_explicitly_marked_stale(self):
        self.cli("--json", "search", "cached", "--project", "fixture")
        Handler.failed = True
        result = self.cli("--json", "search", "cached", "--project", "fixture")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)["cached"])
        self.assertIn("may be outdated", result.stderr)

    def test_offline_creation_does_not_require_keyring_access(self):
        self.env.pop("CJ_TOKEN")
        self.env["DBUS_SESSION_BUS_ADDRESS"] = f"unix:path={self.base}/missing-bus"
        result = self.cli("--offline", "--json", "plan", "create", "--title", "Offline", "--body", "Queued work")
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["id"], receipt["request_id"])
        self.assertIsNone(receipt["resource_id"])
        self.assertEqual(Handler.calls, [])

    def test_pending_receipts_cannot_be_used_as_resource_ids(self):
        result = self.cli("--offline", "--json", "plan", "create", "--project", "fixture", "--title", "Queued", "--body", "Work")
        receipt = json.loads(result.stdout)["request_id"]
        before = len(list((self.base / "state/codejournal/requests").glob("*.json")))
        for args in [["plan", "update", receipt, "--note", "Progress"], ["plan", "show", receipt[:8]],
                     ["log", "add", "--title", "Work", "--body", "Done", "--plan", receipt],
                     ["supersede", RESOURCE, "--by", receipt]]:
            result = self.cli("--offline", *args)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("not a resource ID", result.stderr)
        self.assertEqual(len(list((self.base / "state/codejournal/requests").glob("*.json"))), before)

    def test_legacy_dependent_writes_recover_through_receipts(self):
        result = self.cli("--offline", "--json", "plan", "create", "--project", "fixture", "--title", "Queued", "--body", "Work")
        receipt = json.loads(result.stdout)["request_id"]
        queue = self.base / "state/codejournal/requests"
        item = json.loads(next(queue.glob("*.json")).read_text())
        item.update(id="bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", method="PATCH",
                    path=f"/api/v1/tenants/demo/plans/{receipt}", body={"note": "Progress"})
        (queue / "99999999999999999999-dependent.json").write_text(json.dumps(item))
        result = self.cli("sync")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(any(method == "PATCH" and path.endswith(RESOURCE) for method, path, _ in Handler.calls))
        self.assertEqual(list(queue.glob("*.json")), [])
        result = self.cli("--json", "outbox", "receipt", receipt)
        self.assertEqual(json.loads(result.stdout)["resource_id"], RESOURCE)

    def test_legacy_supersession_resolves_only_the_reference_field(self):
        result = self.cli("--offline", "--json", "add", "--project", "fixture", "--kind", "gotcha", "--title", "Queued", "--body", "Work")
        receipt = json.loads(result.stdout)["request_id"]
        queue = self.base / "state/codejournal/requests"
        item = json.loads(next(queue.glob("*.json")).read_text())
        item.update(id="bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", method="PATCH",
                    path="/api/v1/tenants/demo/entries/12345678",
                    body={"action": "supersede", "by": receipt, "note": receipt})
        (queue / "99999999999999999999-dependent.json").write_text(json.dumps(item))
        result = self.cli("sync")
        self.assertEqual(result.returncode, 0, result.stderr)
        body = next(body for method, _, body in Handler.calls if method == "PATCH")
        self.assertEqual(body["by"], RESOURCE)
        self.assertEqual(body["note"], receipt)

    def test_wrong_resource_receipts_leave_the_request_pending(self):
        result = self.cli("--offline", "--json", "plan", "create", "--project", "fixture", "--title", "Queued", "--body", "Work")
        queue = self.base / "state/codejournal/requests"
        item_path = next(queue.glob("*.json"))
        item = json.loads(item_path.read_text())
        old = "cccccccc-cccc-4ccc-8ccc-cccccccccccc"
        item.update(method="PATCH", path=f"/api/v1/tenants/demo/plans/{old}", body={"note": "Progress"})
        item_path.write_text(json.dumps(item))
        Handler.receipts[old] = {"resource_type": "entries", "resource_id": RESOURCE}
        result = self.cli("sync")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not a plans resource", result.stderr)
        self.assertEqual(json.loads(item_path.read_text()), item)
        self.assertFalse(any(method == "PATCH" for method, _, _ in Handler.calls))
