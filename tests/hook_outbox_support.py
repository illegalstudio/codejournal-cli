import hashlib
import json
import subprocess
import uuid
from auto_sync_support import AutoSyncCase


class HookOutboxCase(AutoSyncCase):
    def setUp(self):
        super().setUp()
        self.env.update(CODE_JOURNAL_AUTO_SYNC="off", CODE_JOURNAL_HOOK_FLUSH="on")
        self.folder = self.base / "state/codejournal/outbox"
        self.folder.mkdir(parents=True, exist_ok=True)

    def origin(self, server=None, token="synthetic-token", tenant="demo"):
        return hashlib.sha256(json.dumps([server or self.server_url, tenant, token],
                                        separators=(",", ":")).encode()).hexdigest()

    def save(self, origin=None, ident=None, **fields):
        event = {"id": ident or str(uuid.uuid4()), "origin": origin, "type": "waiting",
                 "agent": "codex", "project": "fixture", "project_explicit": True,
                 "ts": "2026-10-17T22:23:59Z", **fields}
        path = self.folder / (event["id"] + ".json")
        path.write_text(json.dumps(event, indent=2))
        return path, event

    def snapshot(self):
        return {path.name: path.read_bytes() for path in self.folder.glob("*.json")}

    def capture(self):
        snapshot = {"id": str(uuid.uuid4()), "cwd": str(self.base), "status": "running"}
        subprocess.run([self.binary, "hook", "delegation", "created"], cwd=self.base,
                       env=self.env, input=json.dumps({"project": "fixture", "delegation": snapshot}),
                       capture_output=True, text=True, check=True, timeout=15)
        return json.loads(next(self.folder.glob("*.json")).read_text())
