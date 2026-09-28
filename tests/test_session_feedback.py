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

    def log_message(self, *_args):
        pass

    def do_POST(self):
        size = int(self.headers["Content-Length"])
        body = json.loads(self.rfile.read(size))
        self.calls.append((self.path, body))
        data = json.dumps({"ok": True}).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


class SessionFeedbackTest(unittest.TestCase):
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
        Handler.calls.clear()
        self.temp = tempfile.TemporaryDirectory(prefix="cj-session-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.repo = self.base / "repo"
        self.repo.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.test")
        self.git("config", "user.name", "Test")
        (self.repo / "README.md").write_text("initial\n")
        self.git("add", "README.md")
        self.git("commit", "-m", "initial")
        config = self.base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(self.base / "config"),
                        XDG_STATE_HOME=str(self.base / "state"), CJ_TOKEN="test-token",
                        CJ_PROJECT="p", CLAUDE_CODE_SESSION_ID="session-one")

    def git(self, *args, cwd=None):
        return subprocess.run(["git", *args], cwd=cwd or self.repo, check=True,
                              capture_output=True, text=True).stdout.strip()

    def cli(self, *args, payload=None, cwd=None, session="session-one"):
        env = dict(self.env, CLAUDE_CODE_SESSION_ID=session)
        return subprocess.run([self.binary, *args], cwd=cwd or self.repo, env=env,
                              input=json.dumps(payload) if payload else None, text=True,
                              capture_output=True, timeout=5, check=True).stdout

    def hook(self, event, session, command=None, output=None):
        payload = {"session_id": session, "cwd": str(self.repo)}
        if command:
            payload.update(tool_name="Bash", tool_input={"command": command},
                           tool_response={"stdout": output, "exit_code": 0})
        self.cli("hook", event, payload=payload, session=session)

    def test_amended_worktree_commit_belongs_only_to_its_session(self):
        worktree = self.base / "worktree"
        self.git("worktree", "add", "-b", "session", str(worktree))
        self.hook("SessionStart", "session-one")
        self.hook("SessionStart", "session-two")
        (worktree / "mine.txt").write_text("mine\n")
        self.git("add", "mine.txt", cwd=worktree)
        first = self.git("commit", "-m", "session work", cwd=worktree)
        self.hook("PostToolUse", "session-one", f"git -C {worktree} commit -m 'session work'", first)
        amended = self.git("commit", "--amend", "-m", "amended work", cwd=worktree)
        self.hook("PostToolUse", "session-one", f"git -C {worktree} commit --amend -m 'amended work'", amended)
        expected = self.git("rev-parse", "HEAD", cwd=worktree)
        (self.repo / "other.txt").write_text("other\n")
        self.git("add", "other.txt")
        other = self.git("commit", "-m", "other session")
        self.hook("PostToolUse", "session-two", "git commit -m 'other session'", other)
        self.cli("log", "--title", "My work", "--body", "Done")
        logs = [body for path, body in Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [{"kind": "commit", "value": expected}])
        self.cli("log", "--title", "Follow-up", "--body", "Talked")
        logs = [body for path, body in Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [])

    def test_hook_publishes_checkout_activity_without_waiting_for_network(self):
        self.hook("SessionStart", "session-one")
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if any(path.endswith("/checkout-activity") for path, _ in Handler.calls):
                break
            time.sleep(0.05)
        self.assertTrue(any(path.endswith("/checkout-activity") for path, _ in Handler.calls))

    def test_hook_installer_preserves_existing_settings(self):
        codex_home = self.base / "codex"
        codex_home.mkdir()
        settings = codex_home / "hooks.json"
        settings.write_text(json.dumps({"theme": "dark", "hooks": {"PostToolUse": [
            {"hooks": [{"type": "command", "command": "other-hook"}]}
        ]}}))
        env = dict(self.env, CODEX_HOME=str(codex_home))
        def call(*args):
            return json.loads(subprocess.run([self.binary, "hooks", *args, "--agent", "codex"],
                                             env=env, capture_output=True, text=True, check=True).stdout)
        installed = call("install")
        self.assertTrue(installed["agents"][0]["changed"])
        current = json.loads(settings.read_text())
        self.assertEqual(current["theme"], "dark")
        self.assertEqual(len(current["hooks"]["PostToolUse"]), 2)
        self.assertEqual(len(call("status")["agents"][0]["installed"]), 2)
        self.assertFalse(call("install")["agents"][0]["changed"])
        call("uninstall")
        self.assertEqual(json.loads(settings.read_text())["hooks"]["PostToolUse"][0]
                         ["hooks"][0]["command"], "other-hook")


if __name__ == "__main__":
    unittest.main()
