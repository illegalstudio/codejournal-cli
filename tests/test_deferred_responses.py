import http.server
import json
import os
import pathlib
import re
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
VERSION = re.search(r'^version = "([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M).group(1)
UPGRADE = f"cj {VERSION} is no longer supported by this server. Install cj 9.0.0 or newer, then retry."
LIMITED = "This workspace's plan allows 60 API requests per minute. Retry in 30 seconds."
REFUSALS = {426: {"message": UPGRADE, "minimum_version": "9.0.0"},
            429: {"message": LIMITED, "limit": 60, "retry_after": 30}}


class Handler(http.server.BaseHTTPRequestHandler):
    headers_seen = []
    writes = []
    refusal = None

    def log_message(self, *_args):
        pass

    def respond(self, status, payload):
        self.headers_seen.append((self.headers.get("X-Cj-Version"), self.headers.get("User-Agent")))
        if self.refusal:
            status, payload = self.refusal, REFUSALS[self.refusal]
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
        if not self.refusal:
            self.writes.append((self.headers.get("Idempotency-Key"), body))
        self.respond(201, {"entry": {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"}})


class DeferredResponseTest(unittest.TestCase):
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
        Handler.headers_seen.clear()
        Handler.writes.clear()
        Handler.refusal = None
        self.temp = tempfile.TemporaryDirectory(prefix="cj-deferred-")
        self.addCleanup(self.temp.cleanup)
        base = pathlib.Path(self.temp.name)
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"),
                        XDG_STATE_HOME=str(base / "state"), CJ_TOKEN="test-token")
        self.base = base

    def cli(self, *args, check=True):
        return subprocess.run([self.binary, "--json", "--project", "fixture", *args],
            cwd=self.base, env=self.env, capture_output=True, text=True, timeout=10, check=check)

    def test_requests_carry_the_release(self):
        self.cli("projects")
        version, agent = Handler.headers_seen[0]
        self.assertEqual(version, VERSION)
        self.assertTrue(agent.startswith(f"cj/{VERSION} ("))

    def test_refused_read_without_cache_shows_the_server_message(self):
        for status, message in ((426, UPGRADE), (429, LIMITED)):
            with self.subTest(status=status):
                Handler.refusal = status
                result = self.cli("projects", check=False)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)

    def test_refused_read_falls_back_to_the_cache_with_a_warning(self):
        online = json.loads(self.cli("projects").stdout)
        for status, message in ((426, UPGRADE), (429, LIMITED)):
            with self.subTest(status=status):
                Handler.refusal = status
                result = self.cli("projects")
                self.assertEqual(json.loads(result.stdout), online)
                self.assertIn(f"warning: {message}", result.stderr)

    def test_refused_write_stays_queued_for_sync(self):
        for status, message in ((426, UPGRADE), (429, LIMITED)):
            with self.subTest(status=status):
                Handler.writes.clear()
                Handler.refusal = status
                result = self.cli("add", "--kind", "gotcha", "--title", "Kept note", "--body", "Test")
                queued = json.loads(result.stdout)
                self.assertTrue(queued["queued"])
                self.assertIn(f"warning: {message}", result.stderr)
                self.assertEqual(Handler.writes, [])
                Handler.refusal = None
                self.assertEqual(json.loads(self.cli("sync").stdout)["flushed"], 1)
                self.assertEqual(Handler.writes[0][0], queued["id"])
