import json
import os
from auto_sync_support import Handler
from hook_outbox_support import HookOutboxCase


class HookOutboxRecoveryTest(HookOutboxCase):
    def test_inventory_and_drop_work_without_authentication_and_preserve_writes(self):
        queued = self.enqueue()
        secret = "ghp_" + "Z9y8" * 9
        path, event = self.save(detail=secret)
        self.save(self.origin())
        before = self.snapshot()
        self.config.unlink()
        self.env.pop("CJ_TOKEN")
        listed = self.cli("outbox", "hooks", "list")["hook_events"]
        unbound = next(item for item in listed if item["id"] == event["id"])
        self.assertEqual((unbound["binding"], unbound["reason"]), ("unbound", "unknown"))
        self.assertNotIn("body", unbound)
        bodies = self.cli("outbox", "hooks", "list", "--body")
        self.assertNotIn(secret, json.dumps(bodies))
        self.assertEqual(self.snapshot(), before)
        result = self.cli("outbox", "hooks", "drop", event["id"][:8].upper())
        self.assertEqual(result["dropped_hook_event"], event["id"])
        self.assertFalse(path.exists())
        self.assertEqual(len(self.snapshot()), 1)
        self.assertEqual(self.cli("outbox", "list")["requests"][0]["id"], queued["id"])

    def test_binding_requires_confirmation_and_preserves_original_payload_until_acknowledged(self):
        path, event = self.save(origin_unavailable_reason="credentials_unavailable")
        before = path.read_bytes()
        failed = self.cli("outbox", "hooks", "bind", event["id"], check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("--to-current-account", failed.stderr)
        self.assertEqual(path.read_bytes(), before)
        result = self.cli("outbox", "hooks", "bind", event["id"][:8], "--to-current-account")
        self.assertEqual((result["bound"], result["server"], result["tenant"], result["synced"]),
                         (event["id"], self.server_url, "demo", False))
        rebound = dict(event, origin=self.origin())
        rebound.pop("origin_unavailable_reason")
        self.assertEqual(json.loads(path.read_text()), rebound)
        if os.name == "posix":
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        self.assertEqual(Handler.hook_calls, [])
        Handler.failures[:] = [503]
        bound_bytes = path.read_bytes()
        self.assertNotEqual(self.cli("sync", check=False).returncode, 0)
        self.assertEqual(path.read_bytes(), bound_bytes)
        self.cli("sync")
        self.assertFalse(path.exists())
        expected = dict(event)
        expected.pop("origin")
        expected.pop("origin_unavailable_reason")
        self.assertEqual([call["events"][0] for call in Handler.hook_calls], [expected, expected])

    def test_already_bound_events_cannot_be_reassigned_to_another_account(self):
        path, event = self.save(self.origin(token="another-account"))
        before = path.read_bytes()
        result = self.cli("outbox", "hooks", "bind", event["id"], "--to-current-account", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("already bound", result.stderr)
        self.assertEqual(path.read_bytes(), before)

    def test_binding_uses_the_explicit_server_and_requires_its_credentials(self):
        path, event = self.save()
        before = path.read_bytes()
        self.env.pop("CJ_TOKEN")
        args = ["--server", "https://other.example", "outbox", "hooks", "bind", event["id"],
                "--to-current-account"]
        result = self.cli(*args, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("stored token belongs", result.stderr)
        self.assertEqual(path.read_bytes(), before)
        self.env["CJ_TOKEN"] = "other-account-token"
        result = self.cli(*args)
        self.assertEqual((result["server"], result["tenant"]), ("https://other.example", "demo"))
        self.assertEqual(json.loads(path.read_text())["origin"],
                         self.origin(server="https://other.example", token="other-account-token"))
        self.assertEqual(Handler.hook_calls, [])

    def test_ambiguous_invalid_missing_and_unconfigured_mutations_preserve_files(self):
        self.save(ident="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
        self.save(ident="aaaaaaaa-bbbb-4bbb-8bbb-bbbbbbbbbbbb")
        before = self.snapshot()
        for ident in ["aaaaaaaa", "bad", "../anything", "ffffffff"]:
            for action in ["drop", "bind"]:
                extra = ["--to-current-account"] if action == "bind" else []
                result = self.cli("outbox", "hooks", action, ident, *extra, check=False)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.snapshot(), before)
        self.config.unlink()
        result = self.cli("outbox", "hooks", "bind", "aaaaaaaa-aaaa", "--to-current-account", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.snapshot(), before)

    def test_mutations_refuse_to_run_while_delivery_holds_the_flush_lock(self):
        if os.name != "posix":
            self.skipTest("POSIX lock regression")
        import fcntl
        _, event = self.save()
        before = self.snapshot()
        with (self.folder / ".flush.lock").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            for action, extra in [("drop", []), ("bind", ["--to-current-account"])]:
                result = self.cli("outbox", "hooks", action, event["id"], *extra, check=False)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("synchronization is running", result.stderr)
                self.assertEqual(self.snapshot(), before)
        self.assertEqual(list(self.folder.glob(".tmp*")), [])
