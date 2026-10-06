import http.server
import json
import os
import socket
import subprocess
import threading
import time
import unittest
import urllib.request
from opencode_support import OpenCodeCase
from test_session_feedback import Handler


@unittest.skipUnless(os.environ.get("CJ_TEST_OPENCODE"), "set CJ_TEST_OPENCODE to an OpenCode executable")
class OpenCodeNativeTest(OpenCodeCase):
    def test_native_loader_and_real_plugin_hooks_without_model_generation(self):
        self.setup()
        subprocess.run(["git", "init", "-q", str(self.home)], check=True)
        Handler.calls.clear()
        backend = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=backend.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(thread.join, 2)
        self.addCleanup(backend.server_close)
        self.addCleanup(backend.shutdown)
        self.write(".config/codejournal/config.json", json.dumps({
            "server": f"http://127.0.0.1:{backend.server_port}", "tenant": "demo"
        }))
        env = dict(self.env, OPENCODE_TEST_HOME=str(self.home), XDG_DATA_HOME=str(self.home / "data"),
                   CJ_TOKEN="test-token", CJ_PROJECT="p", CODE_JOURNAL_HOOK_SYNC="1",
                   OPENCODE_DISABLE_PROJECT_CONFIG="true", OPENCODE_CONFIG_CONTENT='{"model":"opencode/big-pickle"}')
        for name in ["OPENCODE_SERVER_PASSWORD", "OPENCODE_SERVER_USERNAME", "OPENCODE_CONFIG", "CODE_JOURNAL_HOOKS"]:
            env.pop(name, None)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        with (self.home / "opencode-test.log").open("w+") as log:
            process = subprocess.Popen([os.environ["CJ_TEST_OPENCODE"], "serve", "--hostname", "127.0.0.1",
                                        "--port", str(port)], cwd=self.home, env=env, stdout=log, stderr=log)
            try:
                base = f"http://127.0.0.1:{port}"
                for _ in range(100):
                    try:
                        with urllib.request.urlopen(base + "/global/health", timeout=1):
                            break
                    except OSError:
                        if process.poll() is not None:
                            log.seek(0)
                            self.fail(log.read()[-4000:])
                        time.sleep(0.1)
                else:
                    self.fail("OpenCode server did not become ready")

                def request(path, body=None):
                    data = None if body is None else json.dumps(body).encode()
                    req = urllib.request.Request(base + path, data=data, headers={"Content-Type": "application/json"})
                    with urllib.request.urlopen(req, timeout=30) as result:
                        return json.load(result)

                config = request("/config")
                self.assertTrue(any("code-journal.js" in str(item) for item in config["plugin"]))
                session = request("/session", {})
                request(f"/session/{session['id']}/message", {
                    "noReply": True, "parts": [{"type": "text", "text": "Synthetic native plugin check."}]
                })
                for _ in range(50):
                    events = [event for path, body in Handler.calls if path.endswith("/client-events")
                              for event in body["events"]]
                    if any(event.get("type") == "prompt" for event in events):
                        break
                    time.sleep(0.1)
                self.assertTrue(any(event.get("type") == "start" and event.get("agent") == "opencode" for event in events))
                self.assertTrue(any(event.get("type") == "prompt" for event in events))
                self.assertNotIn("Synthetic native plugin check", json.dumps(Handler.calls))
                status = self.hooks("status")["agents"][0]
                self.assertEqual(status["sessions_seen"], 1)
                self.assertIsNotNone(status["last_event_at"])
            finally:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
