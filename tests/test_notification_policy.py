import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class PolicyHandler(http.server.BaseHTTPRequestHandler):
    priorities = []
    priority = "low"

    def log_message(self, *_args):
        pass

    def respond(self, payload):
        data = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"]))
        if self.path == "/ntfy":
            self.priorities.append(self.headers["Priority"])
            self.respond({})
            return
        item = json.loads(body)
        if self.priority is not None:
            item["delivery_policy"] = {"desktop": "normal", "ntfy": self.priority}
        self.respond({"notification": item, "queued": False})


class NotificationPolicyTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), PolicyHandler)
        cls.worker = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.worker.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.worker.join(timeout=2)

    def test_ntfy_obeys_server_priority_and_has_neutral_legacy_fallback(self):
        PolicyHandler.priorities.clear()
        with tempfile.TemporaryDirectory(prefix="cj-policy-") as folder:
            base = pathlib.Path(folder)
            config = base / "config" / "codejournal"
            config.mkdir(parents=True)
            url = f"http://127.0.0.1:{self.server.server_port}"
            (config / "config.json").write_text(json.dumps({"server": url, "tenant": "demo",
                "notifications": {"desktop": False, "ntfy_url": url + "/ntfy"}}))
            env = dict(os.environ, XDG_CONFIG_HOME=str(config.parent), XDG_CACHE_HOME=str(base / "cache"),
                       CJ_TOKEN="test-token")
            for priority in ["low", "high", None]:
                PolicyHandler.priority = priority
                subprocess.run([str(ROOT / "target" / "debug" / "cj"), "--json", "--project", "repo",
                                "notify", "--kind", "error", "--title", "Synthetic failure"],
                               cwd=ROOT, env=env, check=True, capture_output=True, timeout=5)
        self.assertEqual(PolicyHandler.priorities, ["low", "high", "default"])


if __name__ == "__main__":
    unittest.main()
