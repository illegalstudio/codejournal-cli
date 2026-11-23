import json
import unittest
from unittest.mock import patch
from auto_sync_support import AutoSyncCase, Handler

TASK = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
STALE = "a" * 64
CURRENT = "b" * 64


def patch_task(self):
    body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
    Handler.calls.append((self.headers.get("Idempotency-Key"), self.path, body, 0))
    if body.get("based_on") == STALE:
        return self.respond(409, {"message": "Task changed; read it again"})
    self.respond(200, {"task": {"id": TASK, "body_version": "next"}})


class OutboxBlockageTest(AutoSyncCase):
    def edit(self, *content):
        return self.cli("--offline", "task", "edit", TASK[:8], "--base", STALE, *content)

    def test_a_rejected_write_is_named_in_later_receipts_and_repaired_by_rebase(self):
        with patch.object(Handler, "do_PATCH", patch_task, create=True):
            first = self.edit("--append-body", "Synthetic context")
            self.cli("projects")
            self.wait_for(lambda: self.saved().get("blocked"))
            self.assertEqual(self.saved()["blocked_request"], first["id"])
            later = self.cli("add", "--kind", "discovery", "--title", "Later", "--body", "Synthetic")
            behind = later["queued_behind"]
            self.assertEqual((behind["request_id"], behind["ahead"], behind["rejected"]), (first["id"], 1, True))
            self.assertIn("Task changed", behind["error"])
            for part in ["It is not being delivered", f"cj outbox rebase {first['id'][:8]} --base VERSION",
                         f"cj outbox drop {first['id'][:8]}"]:
                self.assertIn(part, later["next"])
            blocked = self.cli("status")["automatic_sync"]["blocked_request"]
            self.assertEqual(blocked["request_id"], first["id"])
            failed = self.cli("sync", check=False)
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn(f"the outbox is blocked by write {first['id'][:8]}", failed.stderr)
            names = [path.name for path in self.queue()]
            result = self.cli("outbox", "rebase", first["id"][:8], "--base", CURRENT)
            self.assertEqual((result["rebased"], result["field"], result["from"], result["to"],
                              result["replaces_body"], result["sent"]),
                             (first["id"], "based_on", STALE, CURRENT, False, False))
            self.assertEqual([path.name for path in self.queue()], names)
            self.cli("sync")
            self.assertEqual(self.queue(), [])
            delivered = [call for call in Handler.calls if call[0] == first["id"]][-1]
            self.assertEqual((delivered[2]["based_on"], delivered[2]["append_body"]), (CURRENT, "Synthetic context"))
            self.assertNotIn("blocked_request", self.cli("status")["automatic_sync"])

    def test_rebase_refuses_writes_without_a_base_and_warns_about_replacements(self):
        plain = self.enqueue()
        refused = self.cli("outbox", "rebase", plain["id"], "--base", CURRENT, check=False)
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("carries no base revision", refused.stderr)
        replacing = self.edit("--body", "Synthetic replacement")
        before = {path.name: path.read_bytes() for path in self.queue()}
        same = self.cli("outbox", "rebase", replacing["id"], "--base", STALE, check=False)
        self.assertIn(f"already based on {STALE}", same.stderr)
        self.assertEqual({path.name: path.read_bytes() for path in self.queue()}, before)
        result = self.cli("outbox", "rebase", replacing["id"], "--base", CURRENT)
        self.assertTrue(result["replaces_body"])
        self.env["CODE_JOURNAL_AUTO_SYNC"] = "off"
        later = self.enqueue("Later")
        self.assertEqual((later["queued_behind"]["ahead"], later["queued_behind"]["rejected"]), (2, False))
        self.assertIn("Background delivery retries when online", later["next"])


if __name__ == "__main__":
    unittest.main()
