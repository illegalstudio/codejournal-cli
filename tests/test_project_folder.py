import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest
import urllib.parse


ROOT = pathlib.Path(__file__).resolve().parents[1]


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    registered = None

    def log_message(self, *_args):
        pass

    def reply(self, status, value):
        data = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        url = urllib.parse.urlparse(self.path)
        self.calls.append(("GET", url.path, None))
        if url.path.endswith("/projects/resolve"):
            query = urllib.parse.parse_qs(url.query)
            path = query.get("path", [""])[0]
            inside = Handler.registered and (path == Handler.registered
                                             or path.startswith(Handler.registered + "/"))
            if query.get("within") == ["1"] and inside:
                return self.reply(200, {"project": {"id": "p", "slug": "notes"}})
            return self.reply(404, {"message": "Project not found"})
        self.reply(200, {"project": {"slug": "notes"}, "counts": {}, "rules": "Notes rule"})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.calls.append(("POST", self.path, body))
        if self.path.endswith("/projects"):
            return self.reply(201, {"project": {"slug": body["slug"]}})
        if self.path.endswith("/brief"):
            return self.reply(200, {"project": {"slug": "notes"}, "counts": {}, "rules": "Notes rule"})
        self.reply(200, {"ok": True})


class ProjectFolderTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target" / "debug" / "cj")
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self):
        Handler.calls.clear()
        Handler.registered = None
        temp = tempfile.TemporaryDirectory(prefix="cj-folder-")
        self.addCleanup(temp.cleanup)
        self.base = pathlib.Path(os.path.realpath(temp.name))
        self.plain = self.base / "scratch"
        self.plain.mkdir()
        config = self.base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = {key: value for key, value in os.environ.items() if key != "CJ_PROJECT"}
        self.env.update(XDG_CONFIG_HOME=str(self.base / "config"), XDG_STATE_HOME=str(self.base / "state"),
                        XDG_CACHE_HOME=str(self.base / "cache"), CJ_TOKEN="test-token",
                        CODE_JOURNAL_HOOK_FLUSH="off", CLAUDE_CODE_SESSION_ID="session-one",
                        GIT_CEILING_DIRECTORIES=str(self.base))

    def cli(self, *args, cwd=None, payload=None):
        return subprocess.run([self.binary, *args], cwd=cwd or self.plain, env=self.env, text=True,
                              input=json.dumps(payload) if payload else None,
                              capture_output=True, timeout=10)

    def created_projects(self):
        return [body for method, path, body in Handler.calls
                if method == "POST" and path.endswith("/projects")]

    def test_plain_directory_refuses_writes_without_creating_a_project(self):
        for args in (["add", "--kind", "gotcha", "--title", "x", "--body", "y"],
                     ["feedback", "add", "--category", "bug", "--title", "x", "--body", "y"],
                     ["rules", "set", "Rule"]):
            result = self.cli(*args)
            self.assertNotEqual(result.returncode, 0, args)
            self.assertIn("no Code Journal project here", result.stderr)
        self.assertEqual(self.created_projects(), [])

    def test_unregistered_sessions_never_request_a_work_log(self):
        payload = {"session_id": "session-one", "cwd": str(self.plain)}
        self.cli("hook", "SessionStart", payload=payload)
        self.cli("hook", "PostToolUse", payload={**payload, "tool_name": "Write", "tool_input": {"file_path": "draft.txt"}})
        self.assertEqual(self.cli("hook", "Stop", payload=payload).stdout, "")
        queued = self.base / "state/codejournal/outbox"
        self.assertEqual(list(queued.glob("*.json")), [])

    def test_registered_folder_hook_events_keep_the_project(self):
        Handler.registered = str(self.plain)
        payload = {"session_id": "session-one", "cwd": str(self.plain)}
        self.cli("hook", "SessionStart", payload=payload)
        self.cli("hook", "PostToolUse", payload={**payload, "tool_name": "Write", "tool_input": {"file_path": "draft.txt"}})
        state = json.loads(next((self.base / "state/codejournal/sessions").glob("*.json")).read_text())
        self.assertEqual(state["project"], "notes")
        self.assertIn("cj log add", self.cli("hook", "Stop", payload=payload).stdout)

    def test_session_hook_in_plain_directory_injects_a_notice(self):
        payload = {"session_id": "session-one", "cwd": str(self.plain)}
        result = self.cli("hook", "SessionStart", payload=payload)
        context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("Code Journal: no project here", context)
        self.assertNotIn("Notes rule", context)
        self.assertEqual(self.created_projects(), [])

    def test_subdirectory_of_a_registered_folder_uses_its_project(self):
        Handler.registered = str(self.plain)
        nested = self.plain / "drafts"
        nested.mkdir()
        result = self.cli("--json", "brief", cwd=nested)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("/api/v1/tenants/demo/projects/notes/brief", [path for _, path, _ in Handler.calls])
        payload = {"session_id": "session-one", "cwd": str(nested)}
        context = json.loads(self.cli("hook", "SessionStart", cwd=nested, payload=payload).stdout)
        self.assertIn("Notes rule", context["hookSpecificOutput"]["additionalContext"])

    def test_project_init_registers_a_plain_folder(self):
        result = self.cli("--json", "project", "init")
        self.assertEqual(result.returncode, 0, result.stderr)
        [body] = self.created_projects()
        self.assertEqual((body["slug"], body["kind"], body["path"]),
                         ("scratch", "folder", str(self.plain)))


if __name__ == "__main__":
    unittest.main()
