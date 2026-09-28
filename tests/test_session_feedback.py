import http.server
import json
import os
import pathlib
import socket
import subprocess
import tempfile
import threading
import time
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    canonical = None

    def log_message(self, *_args):
        pass

    def do_POST(self):
        size = int(self.headers["Content-Length"])
        body = json.loads(self.rfile.read(size))
        self.calls.append((self.path, body))
        if self.path.endswith("/client-events"):
            response = {"acknowledged": [event["id"] for event in body["events"]]}
        elif self.path.endswith("/projects"):
            response = {"project": {"slug": Handler.canonical or body["slug"]}}
        elif self.path.endswith("/brief"):
            response = {"project": {"slug": "p"}, "counts": {}, "rules": "Test rule"}
        else:
            response = {"ok": True}
        data = json.dumps(response).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        data = json.dumps({"project": {"slug": "p"}, "counts": {}, "rules": "Test rule"}).encode()
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
        Handler.canonical = None
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
        self.cli("log", "add", "--title", "My work", "--body", "Done")
        logs = [body for path, body in Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [{"kind": "commit", "value": expected}])
        self.cli("log", "add", "--title", "Follow-up", "--body", "Talked")
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

    def test_offline_brief_reuses_cached_online_brief_across_sessions(self):
        self.hook("SessionStart", "session-one")
        result = json.loads(self.cli("--offline", "--json", "brief", "--session-key", "session-two"))
        self.assertEqual(result["rules"], "Test rule")

    def test_delegation_hook_flushes_lifecycle_event_without_session(self):
        self.env["CODE_JOURNAL_HOOK_SYNC"] = "1"
        delegation_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
        self.cli("hook", "delegation", "question", payload={
            "delegation": {"id": delegation_id, "cwd": str(self.repo),
                           "target": "claude", "status": "running"},
            "sender": "guest", "detail": "Synthetic question"})
        events = [item for path, body in Handler.calls if path.endswith("/client-events")
                  for item in body["events"]]
        self.assertEqual(events[0]["type"], "delegation")
        self.assertEqual(events[0]["delegation_id"], delegation_id)
        self.assertEqual(events[0]["kind"], "question")

    def test_hook_events_are_acknowledged_and_removed_from_outbox(self):
        self.env["CODE_JOURNAL_HOOK_SYNC"] = "1"
        self.hook("SessionStart", "session-one")
        self.hook("UserPromptSubmit", "session-one")
        self.cli("hook", "PostToolUse", payload={"session_id": "session-one",
            "cwd": str(self.repo), "tool_name": "Edit", "tool_input": {"file_path": "README.md"}})
        self.cli("hook", "PermissionRequest", payload={"session_id": "session-one",
            "cwd": str(self.repo), "tool_name": "Bash"})
        self.hook("SessionEnd", "session-one")
        events = [event for path, body in Handler.calls if path.endswith("/client-events")
                  for event in body["events"]]
        self.assertEqual([event["type"] for event in events],
                         ["start", "prompt", "edit", "waiting", "end"])
        self.assertEqual(events[2]["file"], "README.md")
        self.assertFalse(list((self.base / "state").rglob("outbox/*.json")))

    def test_queued_hook_event_resolves_checkout_before_delivery(self):
        Handler.canonical = "repo-2"
        outbox = self.base / "state" / "codejournal" / "outbox"
        outbox.mkdir(parents=True)
        event = {"id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", "type": "start",
                 "session": "collision", "agent": "codex", "project": "repo",
                 "project_explicit": False, "host": socket.gethostname(), "cwd": str(self.repo),
                 "checkout_path": str(self.repo), "ts": "2026-09-28T12:00:00Z"}
        (outbox / "0001.json").write_text(json.dumps(event))
        self.cli("sync")
        project = next(body for path, body in Handler.calls if path.endswith("/projects"))
        delivered = next(body["events"][0] for path, body in Handler.calls
                         if path.endswith("/client-events"))
        self.assertEqual((project["path"], project["kind"]), (str(self.repo), "main"))
        self.assertEqual(delivered["project"], "repo-2")

    def test_hook_warns_about_edit_collision_and_reminds_once(self):
        self.env["CODE_JOURNAL_HOOK_FLUSH"] = "off"
        self.hook("SessionStart", "session-one")
        self.hook("SessionStart", "session-two")
        edited = {"session_id": "session-two", "cwd": str(self.repo),
                  "tool_name": "Edit", "tool_input": {"file_path": "README.md"}}
        self.cli("hook", "PostToolUse", payload=edited, session="session-two")
        pre = dict(edited, session_id="session-one")
        warning = json.loads(self.cli("hook", "PreToolUse", payload=pre))
        self.assertIn("README.md was also edited", warning["hookSpecificOutput"]["additionalContext"])
        self.cli("hook", "PostToolUse", payload=pre)
        stop = {"session_id": "session-one", "cwd": str(self.repo)}
        reminder = json.loads(self.cli("hook", "Stop", payload=stop))
        self.assertEqual(reminder["decision"], "block")
        self.assertEqual(self.cli("hook", "Stop", payload=stop), "")

    def test_hook_uses_cached_brief_when_server_is_unavailable(self):
        self.env["CODE_JOURNAL_HOOK_FLUSH"] = "off"
        payload = {"session_id": "session-one", "cwd": str(self.repo)}
        online = json.loads(self.cli("hook", "SessionStart", payload=payload))
        self.assertIn("Test rule", online["hookSpecificOutput"]["additionalContext"])
        config = self.base / "config" / "codejournal" / "config.json"
        stored = json.loads(config.read_text())
        stored["server"] = "http://127.0.0.1:1"
        config.write_text(json.dumps(stored))
        cached = json.loads(self.cli("hook", "SessionStart", payload=payload))
        self.assertIn("Test rule", cached["hookSpecificOutput"]["additionalContext"])

    def test_hook_installer_preserves_existing_settings(self):
        codex_home = self.base / "codex"
        codex_home.mkdir()
        settings = codex_home / "hooks.json"
        settings.write_text(json.dumps({"theme": "dark", "hooks": {"PostToolUse": [
            {"hooks": [{"type": "command", "command": "other-hook"}]}
        ]}}))
        env = dict(self.env, CODEX_HOME=str(codex_home))
        def call(*args):
            return json.loads(subprocess.run([self.binary, "--json", "hooks", *args, "--agent", "codex"],
                                             env=env, capture_output=True, text=True, check=True).stdout)
        dry = subprocess.run([self.binary, "hooks", "install", "--agent", "codex", "--dry-run"],
                             env=env, capture_output=True, text=True, check=True)
        self.assertIn("CJ_RUST_HOOK=1", dry.stdout)
        self.assertEqual(json.loads(settings.read_text())["theme"], "dark")
        installed = call("install")
        self.assertTrue(installed["agents"][0]["changed"])
        current = json.loads(settings.read_text())
        self.assertEqual(current["theme"], "dark")
        self.assertEqual(len(current["hooks"]["PostToolUse"]), 2)
        status = call("status")
        self.assertEqual(len(status["agents"][0]["installed"]), 8)
        self.assertEqual(status["agent"], "codex")
        self.assertEqual(status["missing"], [])
        self.assertEqual(status["sessions_seen"], 0)
        self.assertEqual(status["queued_events"], 0)
        self.assertFalse(status["disabled_by_env"])
        self.assertNotIn("Notification", current["hooks"])
        self.assertEqual(current["hooks"]["SessionStart"][0]["hooks"][0]["timeout"], 30)
        self.assertEqual(current["hooks"]["SessionEnd"][0]["hooks"][0]["timeout"], 3)
        human = subprocess.run([self.binary, "hooks", "status", "--agent", "codex"],
                               env=env, capture_output=True, text=True, check=True)
        self.assertIn("Codex:", human.stdout)
        self.assertIn("Code Journal hooks:", human.stdout)
        self.assertFalse(call("install")["agents"][0]["changed"])
        call("uninstall")
        self.assertEqual(json.loads(settings.read_text())["hooks"]["PostToolUse"][0]
                         ["hooks"][0]["command"], "other-hook")

    def test_uninstall_removes_codex_file_when_it_only_contains_our_hooks(self):
        codex_home = self.base / "codex"
        codex_home.mkdir()
        env = dict(self.env, CODEX_HOME=str(codex_home))
        settings = codex_home / "hooks.json"
        for action in ("install", "uninstall"):
            subprocess.run([self.binary, "hooks", action, "--agent", "codex"],
                           env=env, capture_output=True, text=True, check=True)
        self.assertFalse(settings.exists())

    def test_install_replaces_existing_python_hook_without_double_fire(self):
        codex_home = self.base / "codex"
        codex_home.mkdir()
        env = dict(self.env, CODEX_HOME=str(codex_home))
        settings = codex_home / "hooks.json"
        settings.write_text(json.dumps({"hooks": {"Stop": [
            {"hooks": [{"type": "command", "command":
                        "/home/test/code-journal-hook codex Stop"}]},
            {"hooks": [{"type": "command", "command": "unrelated Stop handler"}]},
        ]}}))
        subprocess.run([self.binary, "hooks", "install", "--agent", "codex"],
                       env=env, capture_output=True, text=True, check=True)
        groups = json.loads(settings.read_text())["hooks"]["Stop"]
        commands = [hook["command"] for group in groups for hook in group["hooks"]]
        self.assertEqual(len(commands), 2)
        self.assertEqual(sum("CJ_RUST_HOOK=1" in command for command in commands), 1)
        self.assertEqual(sum("code-journal-hook" in command for command in commands), 0)
        self.assertIn("unrelated Stop handler", commands)

    def test_hook_status_reports_invalid_settings_without_failing(self):
        codex_home = self.base / "codex"
        codex_home.mkdir()
        (codex_home / "hooks.json").write_text("{invalid json")
        env = dict(self.env, CODEX_HOME=str(codex_home))
        status = subprocess.run([self.binary, "--json", "hooks", "status", "--agent", "codex"],
                                env=env, capture_output=True, text=True, check=True)
        payload = json.loads(status.stdout)
        self.assertIn("invalid JSON", payload["error"])
        self.assertEqual(payload["installed"], [])
        self.assertIn("SessionEnd", payload["missing"])
        human = subprocess.run([self.binary, "hooks", "status", "--agent", "codex"],
                               env=env, capture_output=True, text=True, check=True)
        self.assertIn("error:", human.stdout)

    def test_claude_hook_events_match_python_contract(self):
        claude_home = self.base / "claude"
        claude_home.mkdir()
        env = dict(self.env, CLAUDE_CONFIG_DIR=str(claude_home))
        subprocess.run([self.binary, "--json", "hooks", "install", "--agent", "claude"],
                       env=env, capture_output=True, text=True, check=True)
        groups = json.loads((claude_home / "settings.json").read_text())["hooks"]
        self.assertNotIn("PermissionRequest", groups)
        self.assertIn("Notification", groups)
        self.assertEqual(groups["SessionEnd"][0]["hooks"][0]["timeout"], 5)
        self.assertEqual(groups["PreToolUse"][0]["matcher"],
                         "Edit|Write|MultiEdit|NotebookEdit|StrReplace|Delete")

    def test_setup_agents_installs_skill_and_hooks_in_isolated_home(self):
        codex_home = self.base / "codex"
        codex_home.mkdir()
        env = dict(self.env, CODEX_HOME=str(codex_home))
        def call(*args):
            return json.loads(subprocess.run([self.binary, "setup", "agents", *args,
                "--agent", "codex"], env=env, capture_output=True, text=True,
                check=True).stdout)
        call("--dry-run")
        skill = codex_home / "skills" / "code-journal" / "SKILL.md"
        self.assertFalse(skill.exists())
        call()
        self.assertIn("cj brief", skill.read_text())
        self.assertTrue(call("--status")["agents"][0]["skill_installed"])
        call("--uninstall")
        self.assertFalse(skill.exists())


if __name__ == "__main__":
    unittest.main()
