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
BRIEF = {"project": {"slug": "p", "name": "p"}, "counts": {"active": 1}, "rules": "Synthetic rule"}


class Handler(http.server.BaseHTTPRequestHandler):
    release_calls = 0
    tag = "v9.9.9"

    def log_message(self, *_args):
        pass

    def reply(self, value):
        data = json.dumps(value).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/latest":
            Handler.release_calls += 1
            return self.reply({"tag_name": Handler.tag})
        self.reply(BRIEF)

    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", 0)))
        self.reply(BRIEF)


class UpdateCheckTest(unittest.TestCase):
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
        Handler.release_calls = 0
        Handler.tag = "v9.9.9"
        self.temp = tempfile.TemporaryDirectory(prefix="cj-update-check-")
        self.addCleanup(self.temp.cleanup)
        base = pathlib.Path(self.temp.name)
        url = f"http://127.0.0.1:{self.server.server_port}"
        (base / "config/codejournal").mkdir(parents=True)
        (base / "config/codejournal/config.json").write_text(json.dumps({"server": url, "tenant": "demo"}))
        self.state = base / "state/codejournal/update-check.json"
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"), XDG_STATE_HOME=str(base / "state"),
                        XDG_CACHE_HOME=str(base / "cache"), CJ_TOKEN="test-token", CJ_PROJECT="p",
                        CODE_JOURNAL_AUTO_SYNC="off", CODE_JOURNAL_UPDATE_CHECK="on",
                        CODE_JOURNAL_RELEASES_URL=f"{url}/latest")
        self.env.pop("CJ_DELEGATION_ID", None)

    def cli(self, *args):
        result = subprocess.run([self.binary, *args], env=self.env, cwd=ROOT, text=True,
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def status(self):
        return json.loads(self.cli("status", "--json"))["update_available"]

    def wait_for_release(self):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if self.state.exists() and json.loads(self.state.read_text()).get("latest"):
                return
            time.sleep(0.05)
        self.fail("the background release check did not finish")

    def test_one_background_check_a_day_feeds_status_and_brief(self):
        self.assertIsNone(self.status())
        self.wait_for_release()
        self.assertEqual(self.status(), {"current": "0.1.0", "latest": "9.9.9", "command": "cj update"})
        line = "Update available: cj 9.9.9 is released (installed 0.1.0). Tell the user; they can update with `cj update`."
        self.assertIn(line, self.cli("status"))
        self.assertIn(line, self.cli("brief"))
        self.assertEqual(json.loads(self.cli("brief", "--json"))["update_available"]["latest"], "9.9.9")
        self.assertEqual(Handler.release_calls, 1)
        saved = json.loads(self.state.read_text())
        saved.update(checked_at=saved["checked_at"] - 25 * 3600, latest="0.1.0")
        self.state.write_text(json.dumps(saved))
        self.assertIsNone(self.status())
        self.wait_for_release()
        deadline = time.monotonic() + 10
        while Handler.release_calls < 2 and time.monotonic() < deadline:
            time.sleep(0.05)
        self.assertEqual(Handler.release_calls, 2)

    def test_development_builds_opt_outs_and_guests_never_ask(self):
        for change in [{"CODE_JOURNAL_UPDATE_CHECK": "off"}, {"CODE_JOURNAL_UPDATE_CHECK": None},
                       {"CJ_DELEGATION_ID": "synthetic"}]:
            with self.subTest(change=change):
                env = dict(self.env)
                for key, value in change.items():
                    env.pop(key, None) if value is None else env.update({key: value})
                self.env, saved = env, self.env
                self.state.parent.mkdir(parents=True, exist_ok=True)
                self.state.write_text(json.dumps({"checked_at": 0, "latest": "9.9.9"}))
                self.assertIsNone(self.status())
                self.assertNotIn("Update available", self.cli("brief"))
                self.env = saved
        time.sleep(0.5)
        self.assertEqual(Handler.release_calls, 0)


if __name__ == "__main__":
    unittest.main()
