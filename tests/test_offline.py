import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]


class Handler(http.server.BaseHTTPRequestHandler):
    requests = []
    fail_once = False

    def log_message(self, *_args):
        pass

    def respond(self, status, payload):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/api/v1/me":
            self.respond(200, {"tenant": {"slug": "demo"}})
        elif self.path.endswith("/projects"):
            self.respond(200, {"projects": [{"slug": "fixture", "name": "Fixture",
                "active_entries": 0, "has_rules": False}]})
        else:
            self.respond(404, {"error": "missing"})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.requests.append((self.headers.get("Idempotency-Key"), self.path, body))
        if self.fail_once:
            type(self).fail_once = False
            self.respond(503, {"error": "transient"})
        else:
            self.respond(201, {"entry": {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"}})


class OfflineTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target" / "debug" / "cj")
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.worker = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.worker.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.worker.join(timeout=2)

    def setUp(self):
        Handler.requests.clear()
        Handler.fail_once = False
        self.temp = tempfile.TemporaryDirectory(prefix="cj-offline-")
        self.addCleanup(self.temp.cleanup)
        base = pathlib.Path(self.temp.name)
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"),
                        XDG_STATE_HOME=str(base / "state"), CJ_TOKEN="test-token")
        self.base = base

    def cli(self, *args):
        return subprocess.run([self.binary, "--json", "--project", "fixture", *args],
            cwd=self.base, env=self.env, capture_output=True, text=True, timeout=10, check=True).stdout

    def test_offline_read_uses_cached_response(self):
        online = json.loads(self.cli("projects"))
        offline = json.loads(self.cli("--offline", "projects"))
        self.assertEqual(online, offline)

    def test_offline_write_replays_once(self):
        queued = json.loads(self.cli("--offline", "add", "--kind", "gotcha",
            "--title", "Fixture note", "--body", "Test"))
        self.assertTrue(queued["queued"])
        self.assertEqual(Handler.requests, [])
        result = json.loads(self.cli("sync"))
        self.assertEqual(result["flushed"], 1)
        self.assertEqual(Handler.requests[0][0], queued["id"])
        self.assertEqual(Handler.requests[0][2]["title"], "Fixture note")
        self.assertEqual(json.loads(self.cli("status"))["pending_outbox"], 0)

    def test_server_failure_queues_same_request_id(self):
        Handler.fail_once = True
        queued = json.loads(self.cli("add", "--kind", "gotcha",
            "--title", "Retry note", "--body", "Test"))
        self.assertTrue(queued["queued"])
        self.cli("sync")
        self.assertEqual(len(Handler.requests), 2)
        self.assertEqual(Handler.requests[0][0], Handler.requests[1][0])
