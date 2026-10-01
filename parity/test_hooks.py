"""Legacy hook queue ingestion retains shared session and Live semantics."""
import datetime
import json
import pathlib
import uuid
from support import ParityCase


class HookParityTest(ParityCase):
    def test_hook_lifecycle_records_session_counters_and_files(self):
        for engine in self.envs:
            state = pathlib.Path(self.envs[engine]["XDG_STATE_HOME"]) / ("code-journal" if engine == "python" else "codejournal")
            state.mkdir(parents=True, exist_ok=True)
            now = datetime.datetime.now(datetime.timezone.utc)
            events = []
            for offset, kind in enumerate(["start", "prompt", "edit", "turn", "compact", "waiting"]):
                event = {"id": str(uuid.uuid4()), "type": kind, "session": "parity-session", "agent": "claude",
                         "cwd": str(self.repo), "host": "parity-host", "ts": (now + datetime.timedelta(seconds=offset)).isoformat(),
                         "file": "README.md", "reason": "Synthetic approval"}
                if engine == "rust":
                    event.update(project="parity-repo", project_explicit=True, checkout_path=str(self.repo))
                events.append(event)
            if engine == "python":
                (state / "hook-events.jsonl").write_text("".join(json.dumps(event) + "\n" for event in events))
            else:
                outbox = state / "outbox"
                outbox.mkdir(exist_ok=True)
                for offset, event in enumerate(events):
                    (outbox / f"{offset:020}-{event['id']}.json").write_text(json.dumps(event))
            self.invoke(engine, "hook-flush", json_mode=False)
            brief = self.invoke(engine, "brief", "--agent", "codex")
            session = next(item for item in brief["other_sessions"] if item["agent"] == "claude")
            self.assertEqual((session["prompts"], session["turns"], session["compactions"]), (1, 1, 1))
            self.assertIn("README.md", session["files"])
            self.assertEqual(session["waiting_reason"], "Synthetic approval")
