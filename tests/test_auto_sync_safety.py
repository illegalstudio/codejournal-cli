import json
import time
from auto_sync_support import AutoSyncCase, Handler


class AutoSyncSafetyTest(AutoSyncCase):
    def test_offline_and_explicit_opt_out_do_not_start_delivery(self):
        queued = self.enqueue()
        self.cli("--offline", "status")
        self.cli("--offline", "projects", check=False)
        self.assertEqual(Handler.calls, [])
        self.env["CODE_JOURNAL_AUTO_SYNC"] = "off"
        status = self.cli("status")
        self.cli("projects")
        self.assertFalse(status["automatic_sync"]["enabled"])
        self.assertEqual(Handler.calls, [])
        self.assertEqual(json.loads(self.queue()[0].read_text())["id"], queued["id"])

    def test_other_server_workspace_or_credentials_are_never_replayed(self):
        queued = self.enqueue()
        path = self.queue()[0]
        original = path.read_text()
        for key, value in [("server", "https://other.example"), ("path", "/api/v1/tenants/other/projects/fixture/entries")]:
            request = json.loads(original)
            request[key] = value
            path.write_text(json.dumps(request))
            self.cli("projects")
            time.sleep(0.1)
            self.assertEqual(Handler.calls, [])
        path.write_text(original)
        self.env["CJ_TOKEN"] = "other-account-token"
        self.cli("projects")
        time.sleep(0.1)
        self.assertEqual(Handler.calls, [])
        self.env["CJ_TOKEN"] = "synthetic-token"
        self.cli("projects")
        self.wait_for(lambda: not self.queue())
        self.assertEqual(Handler.calls[0][0], queued["id"])

    def test_rejected_write_is_visible_and_preserves_the_entire_remaining_queue(self):
        self.enqueue("First")
        self.enqueue("Second")
        before = {path.name: path.read_bytes() for path in self.queue()}
        Handler.failures[:] = [422]
        self.cli("projects")
        self.wait_for(lambda: self.saved().get("blocked"))
        status = self.cli("status")
        self.assertTrue(status["automatic_sync"]["blocked"])
        self.assertIn("synthetic rejection", status["automatic_sync"]["last_error"])
        self.assertEqual(status["pending_writes"], 2)
        for _ in range(3):
            self.cli("projects")
        self.assertEqual(len(Handler.calls), 1)
        self.assertEqual({path.name: path.read_bytes() for path in self.queue()}, before)
        self.cli("sync")
        self.assertEqual(self.queue(), [])
        self.assertEqual([body["title"] for _, body in Handler.applied], ["First", "Second"])

    def test_expired_request_remains_untouched_without_any_network_replay(self):
        self.enqueue()
        path = self.queue()[0]
        request = json.loads(path.read_text())
        request["created_at"] = int(time.time()) - 91 * 86400
        path.write_text(json.dumps(request))
        before = path.read_bytes()
        self.cli("projects")
        self.wait_for(lambda: self.saved().get("blocked"))
        self.assertIn("safe retry window has expired", self.saved()["last_error"])
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(Handler.calls, [])

    def test_evicted_response_preserves_pending_writes_without_automatic_resubmission(self):
        self.enqueue("Accepted before response eviction")
        self.enqueue("Later")
        before = {path.name: path.read_bytes() for path in self.queue()}
        Handler.failures[:] = [410]
        self.cli("projects")
        self.wait_for(lambda: self.saved().get("blocked"))
        self.assertIn("saved response was released", self.saved()["last_error"])
        for _ in range(3):
            self.cli("projects")
        self.assertEqual(len(Handler.calls), 1)
        self.assertEqual(Handler.applied, [])
        self.assertEqual({path.name: path.read_bytes() for path in self.queue()}, before)

    def test_unbound_legacy_queue_blocks_later_writes_until_explicit_verified_sync(self):
        self.enqueue("Legacy")
        path = self.queue()[0]
        request = json.loads(path.read_text())
        request.pop("origin")
        path.write_text(json.dumps(request))
        self.enqueue("Later")
        before = path.read_bytes()
        self.cli("projects")
        self.wait_for(lambda: self.saved().get("blocked"))
        self.assertIn("original account", self.saved()["last_error"])
        self.assertEqual(Handler.calls, [])
        self.assertEqual(path.read_bytes(), before)
        self.cli("sync")
        self.assertEqual([body["title"] for _, body in Handler.applied], ["Legacy", "Later"])
        self.assertEqual(self.queue(), [])
