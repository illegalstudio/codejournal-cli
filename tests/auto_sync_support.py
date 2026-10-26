import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    receipts = {}
    applied = []
    failures = []
    retry_after = None
    canonical = "canonical"
    hook_calls = []
    events_applied = set()

    def log_message(self, *_args):
        pass

    def respond(self, status, value):
        data = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        if self.retry_after is not None and status == 429:
            self.send_header("Retry-After", str(self.retry_after))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if "/requests/" in self.path:
            self.respond(404, {"message": "missing"})
        else:
            self.respond(200, {"projects": [], "watches": [], "tenant": {"slug": "demo"}})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path.endswith("/client-events"):
            self.hook_calls.append(body)
            self.events_applied.update(event["id"] for event in body["events"])
            status = self.failures.pop(0) if self.failures else 200
            self.respond(status, {"acknowledged": [event["id"] for event in body["events"]]})
            return
        key = self.headers.get("Idempotency-Key")
        self.calls.append((key, self.path, body, time.monotonic()))
        status = self.failures.pop(0) if self.failures else 201
        if status == 410:
            self.respond(410, {"error": "idempotency_response_evicted", "applied": True,
                               "message": "This request already succeeded, but its saved response was released."})
            return
        if status in [401, 422, 429]:
            self.respond(status, {"message": "synthetic rejection"})
            return
        if key not in self.receipts:
            if self.path.endswith("/projects"):
                value = {"project": {"slug": self.canonical}}
            else:
                value = {"entry": {"id": key}}
            self.receipts[key] = value
            self.applied.append((self.path, body))
        self.respond(status, self.receipts[key])


class AutoSyncCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target/debug/cj")
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=2)

    def setUp(self):
        Handler.calls.clear()
        Handler.receipts.clear()
        Handler.applied.clear()
        Handler.failures.clear()
        Handler.retry_after = None
        Handler.hook_calls.clear()
        Handler.events_applied.clear()
        self.temp = tempfile.TemporaryDirectory(prefix="cj-auto-sync-")
        self.base = pathlib.Path(self.temp.name)
        config = self.base / "config/codejournal"
        config.mkdir(parents=True)
        self.config = config / "config.json"
        self.server_url = f"http://127.0.0.1:{self.server.server_port}"
        self.config.write_text(json.dumps({"server": self.server_url, "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(self.base / "config"),
            XDG_STATE_HOME=str(self.base / "state"), XDG_CACHE_HOME=str(self.base / "cache"),
            CJ_TOKEN="synthetic-token", CODE_JOURNAL_AUTO_SYNC="on", CODE_JOURNAL_HOOK_FLUSH="off")
        self.env.pop("CJ_SERVER_URL", None)
        self.env.pop("CJ_PROJECT", None)
        self.addCleanup(self.temp.cleanup)
        self.addCleanup(self.stop_workers)

    def stop_workers(self):
        self.config.unlink(missing_ok=True)
        # The worker rechecks its configuration during every bounded wait.
        time.sleep(5.1 if self.saved().get("next_retry_at") else 0.1)

    def cli(self, *args, check=True):
        result = subprocess.run([self.binary, "--json", "--project", "fixture", *args],
            cwd=self.base, env=self.env, capture_output=True, text=True, timeout=15, check=check)
        return (json.loads(result.stdout) if result.stdout else {}) if check else result

    def enqueue(self, title="Synthetic note"):
        return self.cli("--offline", "add", "--kind", "discovery", "--title", title, "--body", "Synthetic")

    def queue(self):
        return sorted((self.base / "state").rglob("requests/*.json"))

    def saved(self):
        paths = list((self.base / "state").rglob("request-sync/*.json"))
        return json.loads(paths[0].read_text()) if paths else {}

    def wait_for(self, condition, timeout=15):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if condition():
                return
            time.sleep(0.05)
        self.fail("automatic synchronization did not reach the expected state")
