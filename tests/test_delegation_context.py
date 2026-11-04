import hashlib
import json
from delegation_support import DelegationCase


READ_CONTEXT = r'''
if os.environ.get("CJ_FAKE_BEHAVIOR") == "context":
    import hashlib
    for item in call("context", delegation_id)["attachments"]:
        offset = 0
        pieces = []
        while True:
            part = call("context", delegation_id, "--name", item["name"], "--offset", str(offset), "--limit", "12000")["attachment"]
            pieces.append(part["body"])
            if part["next_offset"] is None:
                break
            offset = part["next_offset"]
        if hashlib.sha256("".join(pieces).encode()).hexdigest() != item["sha256"]:
            raise RuntimeError("Context snapshot hash mismatch")
'''


class DelegationContextTest(DelegationCase):
    def setUp(self):
        super().setUp()
        fake = self.bin / "claude"
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")', 'publish("ready", "Protocol loaded")\n' + READ_CONTEXT))

    def test_immutable_context_survives_file_changes_and_records_read_ranges(self):
        source = self.root / "ledger.txt"
        content = "immutable evidence €\n" * 6000
        source.write_text(content)
        ident = self.call("create", "claude", "--prompt", "Read the attached evidence", "--context-file", str(source))["id"]
        source.write_text("changed after capture")
        self.call("recover", ident, behavior="context")
        snapshot = self.settled(ident)
        self.assertEqual(snapshot["status"], "completed", json.dumps(snapshot))
        reads = [event["payload"] for event in snapshot["events"] if event["kind"] == "context_read"]
        self.assertGreater(len(reads), 1)
        self.assertEqual(reads[0]["sha256"], hashlib.sha256(content.encode()).hexdigest())
        self.assertEqual(reads[0]["offset"], 0)
        self.assertEqual(reads[-1]["end"], len(content))
        recovered = self.call("context", ident, "--name", "file-1", "--limit", "20")
        self.assertEqual(recovered["attachment"]["body"], content[:20])

    def test_result_is_blocked_until_required_context_has_been_read(self):
        source = self.root / "ledger.txt"
        source.write_text("Required evidence")
        ident = self.call("start", "claude", "--prompt", "Read the evidence", "--context-file", str(source))["id"]
        snapshot = self.settled(ident)
        self.assertEqual(snapshot["status"], "stalled")
        self.assertIn("unread ranges", snapshot["session"]["error"])

    def test_previous_result_can_be_attached_without_cross_session_access(self):
        previous = self.start()
        self.settled(previous)
        ident = self.call("start", "claude", "--prompt", "Recheck the prior result", "--context-result", previous, behavior="context")["id"]
        self.assertEqual(self.settled(ident)["status"], "completed")
        part = self.call("context", ident, "--name", "result-1")["attachment"]
        self.assertEqual(part["source"]["delegation_id"], previous)
        self.assertEqual(part["body"], "Standalone provider completed")

    def test_large_unicode_task_is_loaded_from_canonical_state(self):
        prompt = self.root / "task.txt"
        prompt.write_text("task €" * 16000)
        ident = self.call("start", "claude", "--prompt-file", str(prompt))["id"]
        self.assertEqual(self.settled(ident)["status"], "completed")
        self.assertEqual(self.call("sync", ident, "--full")["session"]["task"], prompt.read_text())
