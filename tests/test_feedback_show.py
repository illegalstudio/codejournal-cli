import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
REPORT = {
    "id": "abcdef12-3456-4000-8000-000000000001", "project_id": "project-id",
    "project_slug": "sample", "agent": "test-agent", "host": "test-host",
    "category": "retrieval", "title": "Read one report", "body": "Full report\nSecond line",
    "status": "done", "resolution": "Fixed with focused retrieval",
    "created_at": "2026-10-09T09:00:00Z", "updated_at": "2026-10-09T10:00:00Z",
}


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    report = REPORT
    status = 200

    def log_message(self, *_args):
        pass

    def do_GET(self):
        self.calls.append(self.path)
        self.send_response(self.status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        payload = {"feedback": self.report} if self.status == 200 else {
            "message": "Feedback ID is missing or ambiguous"
        }
        self.wfile.write(json.dumps(payload).encode())


class FeedbackShowTest(unittest.TestCase):
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
        Handler.report = dict(REPORT)
        Handler.status = 200
        self.temp = tempfile.TemporaryDirectory(prefix="cj-feedback-show-")
        self.addCleanup(self.temp.cleanup)
        config = pathlib.Path(self.temp.name) / "codejournal"
        config.mkdir()
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=self.temp.name, XDG_CACHE_HOME=self.temp.name,
                        XDG_STATE_HOME=self.temp.name, CJ_TOKEN="test-token", CODE_JOURNAL_AUTO_SYNC="off")
        self.env.pop("CJ_PROJECT", None)
        self.env.pop("CJ_SERVER_URL", None)

    def command(self, *args):
        return subprocess.run([self.binary, "feedback", "show", *args], cwd=self.temp.name,
                              env=self.env, text=True, capture_output=True, timeout=3)

    def test_human_detail_uses_one_request_without_a_checkout(self):
        result = self.command("ABCDEF12")
        self.assertEqual(result.returncode, 0, result.stderr)
        for content in [REPORT["id"].replace("-", ""), "project:   sample", "status:    done",
                        "category:  retrieval", "test-agent", "test-host", REPORT["created_at"],
                        REPORT["updated_at"], REPORT["title"], REPORT["body"], REPORT["resolution"]]:
            self.assertIn(content, result.stdout)
        self.assertEqual(Handler.calls, ["/api/v1/tenants/demo/feedback/ABCDEF12"])

    def test_json_preserves_full_detail_for_each_status(self):
        for status in ["open", "done", "dismissed"]:
            with self.subTest(status=status):
                Handler.report["status"] = status
                result = self.command(REPORT["id"], "--json")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(result.stdout), {"feedback": Handler.report})
        self.assertEqual(Handler.calls, [f"/api/v1/tenants/demo/feedback/{REPORT['id']}"] * 3)

    def test_optional_metadata_and_terminal_controls(self):
        Handler.report.update(project_id=None, project_slug=None, agent=None, host=None,
                              resolution=None, body="Report\x1b[2J\nDetails")
        result = self.command("abcdef12")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("project:   no project", result.stdout)
        self.assertIn("agent:     unknown", result.stdout)
        self.assertNotIn("resolution:", result.stdout)
        self.assertNotIn("\x1b", result.stdout)
        self.assertIn("Details", result.stdout)

    def test_lookup_failure_does_not_fall_back_to_a_list(self):
        Handler.status = 422
        result = self.command("abcdef12")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Feedback ID is missing or ambiguous", result.stderr)
        self.assertEqual(Handler.calls, ["/api/v1/tenants/demo/feedback/abcdef12"])
