import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class ActivityHandler(http.server.BaseHTTPRequestHandler):
    calls = []
    receipts = {}
    changes = []
    fail_once = False

    def log_message(self, *_args):
        pass

    def respond(self, status, data):
        encoded = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def do_GET(self):
        self.respond(200, {"tenant": {"slug": "demo"}})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        key = self.headers.get("Idempotency-Key")
        self.calls.append((key, body))
        if key not in self.receipts:
            self.receipts[key] = {"changed": True}
            self.changes.append(body)
        if type(self).fail_once:
            type(self).fail_once = False
            self.respond(503, {"message": "uncertain delivery"})
        else:
            self.respond(200, self.receipts[key])


class ActivityDeliveryTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target/debug/cj")
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), ActivityHandler)
        cls.worker = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.worker.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.worker.join(timeout=2)

    def setUp(self):
        ActivityHandler.calls.clear()
        ActivityHandler.receipts.clear()
        ActivityHandler.changes.clear()
        ActivityHandler.fail_once = False
        self.temp = tempfile.TemporaryDirectory(prefix="cj-activity-delivery-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.repo = self.base / "repo"
        self.repo.mkdir()
        for args in [("init", "-q", "-b", "main"), ("config", "user.name", "Test"),
                     ("config", "user.email", "test@example.test"), ("config", "commit.gpgsign", "false")]:
            self.git(*args)
        (self.repo / "file").write_text("Initial")
        self.git("add", "file")
        self.git("commit", "-qm", "Initial")
        config = self.base / "config/codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(self.base / "config"),
            XDG_STATE_HOME=str(self.base / "state"), XDG_CACHE_HOME=str(self.base / "cache"), CJ_TOKEN="test-token")

    def git(self, *args):
        subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True)

    def cli(self, *args):
        return subprocess.run([self.binary, "--json", "--project", "p", *args], cwd=self.repo,
            env=self.env, text=True, capture_output=True, check=True, timeout=10).stdout

    def queue(self):
        return sorted((self.base / "state").rglob("requests/*.json"))

    def test_identical_delivered_state_skips_posts_and_real_changes_send_immediately(self):
        for _ in range(4):
            self.cli("activity", "publish")
        self.assertEqual(len(ActivityHandler.calls), 1)
        (self.repo / "file").write_text("Changed")
        self.cli("activity", "publish")
        self.assertEqual(len(ActivityHandler.calls), 2)
        confirmation = next((self.base / "state").rglob("activity-delivery/*.json"))
        stored = json.loads(confirmation.read_text())
        stored["acknowledged_at"] = 0
        confirmation.write_text(json.dumps(stored))
        self.cli("activity", "publish")
        self.assertEqual(len(ActivityHandler.calls), 3)
        self.env["CJ_TOKEN"] = "new-token"
        self.cli("activity", "publish")
        self.assertEqual(len(ActivityHandler.calls), 4)

    def test_pending_snapshots_keep_state_transitions_in_order_and_only_skip_latest_identical(self):
        self.cli("--offline", "activity", "publish")
        self.cli("--offline", "activity", "publish")
        self.assertEqual(len(self.queue()), 1)
        (self.repo / "file").write_text("Changed")
        self.cli("activity", "publish")
        self.assertEqual(ActivityHandler.calls, [])
        self.assertEqual(len(self.queue()), 2)
        self.git("checkout", "--", "file")
        self.cli("activity", "publish")
        self.assertEqual(len(self.queue()), 3)
        self.cli("sync")
        self.assertEqual([body["dirty"] for body in ActivityHandler.changes], [0, 1, 0])
        self.assertEqual(self.queue(), [])
        self.cli("activity", "publish")
        self.assertEqual(len(ActivityHandler.calls), 3)

    def test_uncertain_delivery_keeps_original_request_id_until_acknowledged(self):
        ActivityHandler.fail_once = True
        self.cli("activity", "publish")
        path = self.queue()[0]
        queued = json.loads(path.read_text())
        self.cli("activity", "publish")
        self.assertEqual(len(self.queue()), 1)
        self.cli("sync")
        self.assertEqual([key for key, _ in ActivityHandler.calls], [queued["id"], queued["id"]])
        self.assertEqual(len(ActivityHandler.changes), 1)
        self.assertFalse(path.exists())
        self.cli("activity", "publish")
        self.assertEqual(len(ActivityHandler.calls), 2)
