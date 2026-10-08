import json
import uuid
from auto_sync_support import AutoSyncCase, Handler


class AutoSyncHooksTest(AutoSyncCase):
    def test_hook_events_retry_automatically_with_their_original_ids(self):
        self.enqueue()
        request = json.loads(self.queue()[0].read_text())
        self.queue()[0].unlink()
        directory = self.base / "state/codejournal/outbox"
        directory.mkdir(parents=True, exist_ok=True)
        ident = str(uuid.uuid4())
        event = {"id": ident, "origin": request["origin"], "event": "session_end",
                 "project": "fixture", "project_explicit": True}
        (directory / "001.json").write_text(json.dumps(event))
        Handler.failures[:] = [503]
        self.env["CODE_JOURNAL_HOOK_FLUSH"] = "on"
        self.cli("projects")
        self.wait_for(lambda: self.saved().get("next_retry_at") is not None)
        self.wait_for(lambda: not list(directory.glob("*.json")))
        self.assertEqual(len(Handler.hook_calls), 2)
        self.assertEqual(Handler.events_applied, {ident})
        for call in Handler.hook_calls:
            self.assertEqual(call["events"][0]["id"], ident)
            self.assertNotIn("origin", call["events"][0])
