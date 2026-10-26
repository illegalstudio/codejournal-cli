import datetime
import hashlib
import json
import subprocess
import uuid
from auto_sync_support import AutoSyncCase, Handler


class DelegationTimestampsTest(AutoSyncCase):
    def setUp(self):
        super().setUp()
        self.env.update(CODE_JOURNAL_AUTO_SYNC="off", CODE_JOURNAL_HOOKS="on")
        self.folder = self.base / "state/codejournal/outbox"
        self.folder.mkdir(parents=True, exist_ok=True)

    def event(self):
        origin = hashlib.sha256(json.dumps([self.server_url, "demo", "synthetic-token"],
                                          separators=(",", ":")).encode()).hexdigest()
        ident = str(uuid.uuid4())
        return {"id": str(uuid.uuid4()), "origin": origin, "type": "delegation",
                "session": ident, "agent": "delegation", "project": "fixture",
                "project_explicit": True, "ts": "2026-10-17T17:00:00Z",
                "delegation_id": ident, "kind": "created",
                "delegation": {"id": ident, "created_at": 1792256249.125,
                               "updated_at": 1792256250.25, "integrated_at": None,
                               "target": "grok", "mode": "read-only", "status": "running"}}

    def save(self, event, filename="001.json"):
        path = self.folder / filename
        path.write_text(json.dumps(event, indent=2))
        return path

    def assert_time(self, value, expected):
        self.assertIsInstance(value, str)
        parsed = datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))
        self.assertEqual(parsed.utcoffset(), datetime.timedelta())
        self.assertAlmostEqual(parsed.timestamp(), expected, places=6)

    def test_new_broker_event_is_normalized_before_it_is_queued(self):
        event = self.event()
        payload = {"at": 1792256251.5, "project": "fixture", "delegation": event["delegation"]}
        subprocess.run([self.binary, "hook", "delegation", "created"],
                       cwd=self.base, env=self.env, input=json.dumps(payload),
                       capture_output=True, text=True, check=True, timeout=15)
        files = list(self.folder.glob("*.json"))
        self.assertEqual(len(files), 1)
        queued = json.loads(files[0].read_text())
        self.assert_time(queued["ts"], payload["at"])
        for field in ["created_at", "updated_at"]:
            self.assert_time(queued["delegation"][field], event["delegation"][field])
        self.assertIsNone(queued["delegation"]["integrated_at"])
        self.assertEqual(Handler.hook_calls, [])

    def test_legacy_retry_preserves_ids_and_disk_bytes_until_acknowledgement(self):
        original = self.event()
        original["delegation"]["integrated_at"] = 1792256252
        path = self.save(original)
        before = path.read_bytes()
        Handler.failures[:] = [503]
        self.assertNotEqual(self.cli("hook-flush", check=False).returncode, 0)
        self.assertEqual(path.read_bytes(), before)
        self.cli("hook-flush")
        self.assertFalse(path.exists())
        self.assertEqual(len(Handler.hook_calls), 2)
        for call in Handler.hook_calls:
            event = call["events"][0]
            self.assertEqual(event["id"], original["id"])
            self.assertEqual(event["delegation_id"], original["delegation_id"])
            self.assertEqual(event["ts"], original["ts"])
            self.assertEqual(event["delegation"]["status"], "running")
            self.assertNotIn("origin", event)
            for field in ["created_at", "updated_at", "integrated_at"]:
                self.assert_time(event["delegation"][field], original["delegation"][field])

    def test_existing_dates_and_null_fields_are_delivered_unchanged(self):
        original = self.event()
        original["delegation"].update(created_at="2026-10-17T19:00:00+02:00", updated_at=None)
        self.save(original)
        self.cli("hook-flush")
        original.pop("origin")
        self.assertEqual(Handler.hook_calls[0]["events"][0], original)

    def test_other_credentials_are_not_normalized_or_sent(self):
        foreign = self.event()
        foreign.update(origin="other-account")
        foreign["delegation"]["created_at"] = 1e30
        path = self.save(foreign, "000.json")
        before = path.read_bytes()
        original = self.event()
        self.save(original)
        self.cli("hook-flush")
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual([e["id"] for e in Handler.hook_calls[0]["events"]], [original["id"]])

    def test_invalid_unix_date_preserves_the_entire_batch(self):
        invalid = self.event()
        invalid["delegation"]["updated_at"] = 1e30
        self.save(invalid)
        self.save(self.event(), "002.json")
        before = {p.name: p.read_bytes() for p in self.folder.glob("*.json")}
        result = self.cli("hook-flush", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("delegation.updated_at", result.stderr)
        self.assertEqual({p.name: p.read_bytes() for p in self.folder.glob("*.json")}, before)
        self.assertEqual(Handler.hook_calls, [])
