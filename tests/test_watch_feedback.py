import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import time
import unittest
import uuid


ROOT = pathlib.Path(__file__).resolve().parents[1]


class Handler(http.server.BaseHTTPRequestHandler):
    watches = {}
    notifications = []

    def log_message(self, *_args):
        pass

    def respond(self, payload, status=200):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def body(self):
        return json.loads(self.rfile.read(int(self.headers["Content-Length"])))

    def do_POST(self):
        body = self.body()
        ident = str(uuid.uuid4())
        watch = dict(body, id=ident, command=json.dumps(body["command"]), status="running")
        self.watches[ident] = watch
        self.respond({"watch": watch}, 201)

    def do_GET(self):
        if self.path.endswith("/notifications"):
            self.respond({"notifications": self.notifications})
        elif "/watches/" in self.path:
            self.respond({"watch": self.watches[self.path.rsplit("/", 1)[-1]]})
        else:
            all_watches = list(self.watches.values())
            if "all=1" not in self.path:
                all_watches = [watch for watch in all_watches if watch["status"] == "running"]
            self.respond({"watches": all_watches})

    def do_PATCH(self):
        body = self.body()
        watch = self.watches[self.path.rsplit("/", 1)[-1]]
        if watch["status"] != "running":
            self.respond({"watch": watch, "changed": False})
            return
        watch.update(body)
        failed = body["status"] == "timed_out" or body.get("exit_code", 0) != 0
        if body["status"] != "cancelled" and (watch["notify_on"] != "failure" or failed):
            outcome = "timed out" if body["status"] == "timed_out" else "failed" if failed else "succeeded"
            self.notifications.append({"title": f"{watch['title']}: {outcome}"})
        self.respond({"watch": watch, "changed": True})


class WatchFeedbackTest(unittest.TestCase):
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
        Handler.watches.clear()
        Handler.notifications.clear()
        self.temp = tempfile.TemporaryDirectory(prefix="cj-watch-")
        self.addCleanup(self.temp.cleanup)
        config = pathlib.Path(self.temp.name) / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(config.parent),
                        XDG_STATE_HOME=str(pathlib.Path(self.temp.name) / "state"),
                        CJ_TOKEN="test-token")

    def cli(self, *args):
        return subprocess.run([self.binary, "--project", "p", *args], env=self.env,
                              cwd=ROOT, capture_output=True, text=True, timeout=5, check=True).stdout

    def wait(self, ident, status):
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline:
            if Handler.watches[ident]["status"] == status:
                return Handler.watches[ident]
            time.sleep(0.05)
        self.fail(f"watch {ident} did not reach {status}")

    def test_watch_failure_timeout_cancel_and_delivery_message(self):
        failed = json.loads(self.cli("--json", "watch", "start", "--title", "Fake CI", "--",
                                     "sh", "-c", "echo FAILED; exit 3"))["watch"]
        result = self.wait(failed["id"], "finished")
        self.assertEqual(result["exit_code"], 3)
        self.assertIn("FAILED", result["tail"])
        self.assertEqual(Handler.notifications[-1]["title"], "Fake CI: failed")
        quiet = json.loads(self.cli("--json", "watch", "start", "--title", "Quiet", "--notify-on",
                                    "failure", "--", "true"))["watch"]
        self.wait(quiet["id"], "finished")
        self.assertEqual(len(Handler.notifications), 1)
        slow = json.loads(self.cli("--json", "watch", "start", "--title", "Slow", "--timeout", "1",
                                   "--", "sleep", "30"))["watch"]
        self.wait(slow["id"], "timed_out")
        self.assertEqual(Handler.notifications[-1]["title"], "Slow: timed out")
        running = json.loads(self.cli("--json", "watch", "start", "--title", "Long", "--",
                                      "sleep", "30"))["watch"]
        self.cli("watch", "cancel", running["id"][:8])
        self.wait(running["id"], "cancelled")
        self.assertEqual(len(Handler.notifications), 2)
        text = self.cli("watch", "start", "--title", "Message", "--", "true")
        self.assertIn("a notification goes to: dashboard bell.", text)
        self.assertIn("desktop and ntfy delivery are off", text)
        message = next(watch for watch in Handler.watches.values() if watch["title"] == "Message")
        self.wait(message["id"], "finished")


if __name__ == "__main__":
    unittest.main()
