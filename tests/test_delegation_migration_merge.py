from contextlib import closing
import hashlib
import sqlite3
from delegation_support import DelegationCase


class DelegationMigrationMergeTest(DelegationCase):
    def source(self):
        database = self.root/"state/codejournal/delegations/delegations.sqlite3"
        source = self.root/"legacy.sqlite3"
        with closing(sqlite3.connect(database)) as db, closing(sqlite3.connect(source)) as backup:
            db.backup(backup)
        return source

    def test_merge_preserves_native_events_and_translates_legacy_replies_and_acknowledgements(self):
        legacy = self.start("question")
        question = self.wait_for(legacy, "question")
        self.call("answer", legacy, "--reply-to", str(question["seq"]), "--body", "Reviewed")
        self.settled(legacy)
        self.call("ack", legacy, "--through", str(question["seq"]))
        source = self.source()
        before = hashlib.sha256(source.read_bytes()).hexdigest()
        environment = self.env | {"XDG_STATE_HOME": str(self.root/"new-state")}
        native = self.call("create", "claude", "--prompt", "Existing native session", env=environment)["session"]
        current = self.call("status", native["id"], env=environment)
        imported = self.call("migrate", "--from", str(source), "--merge", env=environment)
        offset = imported["sequence_offset"]
        self.assertGreater(offset, 0)
        self.assertEqual(self.call("status", native["id"], env=environment)["events"], current["events"])
        history = self.call("status", legacy, "--full", env=environment)
        reply = next(e for e in history["events"] if e["kind"] == "answer")
        self.assertEqual(reply["reply_to"], question["seq"]+offset)
        self.assertEqual(history["session"]["host_ack_seq"], question["seq"]+offset)
        unread = self.call("sync", legacy, env=environment)
        self.assertTrue(all(e["seq"] > question["seq"]+offset for e in unread["events"]))
        self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(), before)
        self.assertTrue(self.call("migrate", "--from", str(source), "--merge", env=environment)["already_imported"])

    def test_merge_cannot_replace_an_existing_session(self):
        native = self.call("create", "claude", "--prompt", "Preserve this session")["session"]
        source = self.source()
        rejected = self.invoke("migrate", "--from", str(source), "--merge", check=False)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn("already exists", rejected.stderr)
        self.assertEqual(self.call("status", native["id"], "--full")["session"]["task"], "Preserve this session")
