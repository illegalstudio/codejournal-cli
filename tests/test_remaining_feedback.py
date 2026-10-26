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
    calls = []
    malformed = None
    retry = False

    def log_message(self, *_args):
        pass

    def respond(self, status, value):
        data = value.encode() if isinstance(value, str) else json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self.respond(200, {"plan": {"revision": 3, "body": "- [ ] First"}, "project": {"slug": "p"}})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.calls.append((self.path, self.headers.get("Idempotency-Key"), body))
        if self.path.endswith("/projects"):
            self.respond(201, {"project": {"slug": "repo"}})
        elif self.retry:
            type(self).retry = False
            self.respond(404, {"message": "Project not found"})
        elif self.malformed is not None:
            self.respond(self.malformed, "<html>temporary failure</html>")
        else:
            self.respond(201, {"entry": {"id": "a" * 32}})

    def do_PATCH(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.calls.append((self.path, self.headers.get("Idempotency-Key"), body))
        self.respond(200, {"plan": {"revision": 4}})


class RemainingFeedbackTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target/debug/cj")
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.worker = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.worker.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.worker.join(timeout=2)

    def setUp(self):
        Handler.calls.clear()
        Handler.malformed = None
        Handler.retry = False
        self.temp = tempfile.TemporaryDirectory(prefix="cj-feedback-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.repo = self.base / "repo"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        config = self.base / "config/codejournal"
        config.mkdir(parents=True)
        self.url = f"http://127.0.0.1:{self.server.server_port}"
        (config / "config.json").write_text(json.dumps({"server": self.url, "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(self.base / "config"),
                        XDG_STATE_HOME=str(self.base / "state"), XDG_CACHE_HOME=str(self.base / "cache"),
                        CJ_TOKEN="test-token", CODE_JOURNAL_HOOK_FLUSH="off")
        self.env.pop("CJ_PROJECT", None)

    def cli(self, *args, auto=False, check=True, env=None):
        prefix = [] if auto else ["--project", "p"]
        return subprocess.run([self.binary, "--json", *prefix, *args], cwd=self.repo,
            env=env or self.env, text=True, capture_output=True, timeout=10, check=check)

    def add(self, *args, auto=False, check=True):
        return self.cli(*args, "add", "--kind", "gotcha", "--title", "Note", "--body", "Synthetic", auto=auto, check=check)

    def test_invalid_success_and_retry_keep_durable_ids(self):
        for auto in [False, True]:
            Handler.malformed = 200
            Handler.retry = auto
            queued = json.loads(self.add(auto=auto).stdout)
            listed = json.loads(self.cli("outbox", "list").stdout)["requests"]
            self.assertIn(queued["id"], [item["id"] for item in listed])
            calls = [call for call in Handler.calls if call[0].endswith("/entries")]
            self.assertEqual(calls[-1][1], queued["id"])
            if auto:
                self.assertEqual(calls[-2][1], calls[-1][1])
            Handler.malformed = None
            self.cli("sync")
            self.assertEqual(Handler.calls[-1][1], queued["id"])

    def test_malformed_validation_error_is_explicitly_not_queued(self):
        Handler.malformed = 422
        result = self.add(check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("write not queued (request ", result.stderr)
        self.assertEqual(json.loads(self.cli("outbox", "list").stdout)["requests"], [])

    def test_other_servers_do_not_block_sync_and_drop_needs_no_login(self):
        queued = json.loads(self.add("--offline").stdout)
        queue = next((self.base / "state").rglob("requests/*.json"))
        value = json.loads(queue.read_text())
        value["server"] = "http://127.0.0.1:1"
        queue.write_text(json.dumps(value))
        self.assertEqual(json.loads(self.cli("sync").stdout)["flushed"], 0)
        env = dict(self.env, XDG_CONFIG_HOME=str(self.base / "missing"))
        env.pop("CJ_TOKEN")
        listed = json.loads(self.cli("outbox", "list", env=env).stdout)["requests"]
        self.assertEqual(listed[0]["id"], queued["id"])
        self.assertNotIn("body", listed[0])
        self.cli("outbox", "drop", queued["id"], env=env)
        self.assertFalse(queue.exists())

    def test_offline_unknown_move_is_rejected_before_queueing(self):
        result = self.cli("--offline", "doc", "move", "00000000", "--to", "global", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cached, verified source ID", result.stderr)
        self.assertEqual(json.loads(self.cli("outbox", "list").stdout)["requests"], [])

    def test_step_uses_current_revision_and_sends_only_atomic_changes(self):
        self.cli("plan", "step", "a" * 8, "1", "--done", "--note", "Verified")
        body = Handler.calls[-1][2]
        self.assertEqual(body["based_on"], 3)
        self.assertEqual(body["step_index"], 1)
        self.assertTrue(body["step_done"])
        self.assertNotIn("body", body)
