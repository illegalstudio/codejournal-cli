import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
PLAN_ID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []

    def log_message(self, *_args):
        pass

    def respond(self, status, payload):
        content = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(content)))
        self.end_headers()
        self.wfile.write(content)

    def do_GET(self):
        if self.path.endswith("/plans"):
            self.respond(200, {"plans": [{"id": PLAN_ID}]})
        elif self.path.endswith("/docs"):
            self.respond(200, {"docs": [{"id": PLAN_ID}]})
        elif self.path.endswith(f"/plans/{PLAN_ID}") or self.path.endswith("/plans/aaaaaaaa"):
            self.respond(200, {"plan": {"id": PLAN_ID, "body": "## Steps\n- [ ] ship"}, "revisions": []})
        elif self.path.endswith(f"/docs/{PLAN_ID}") or self.path.endswith("/docs/aaaaaaaa"):
            self.respond(200, {"doc": {"id": PLAN_ID, "body": "Documentation body"}, "revisions": []})
        elif self.path.endswith("/tasks?all=1"):
            self.respond(200, {"tasks": [{"id": PLAN_ID}]})
        else:
            self.respond(404, {"error": "unknown route"})

    def record(self):
        size = int(self.headers.get("Content-Length", "0"))
        body = json.loads(self.rfile.read(size))
        self.calls.append((self.command, self.path, body))
        self.respond(200, {"ok": True})

    do_PATCH = record
    do_POST = record
    do_PUT = record


class FeedbackCliTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        binary = ROOT / "target" / "debug" / "cj"
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(binary)
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
        self.temp = tempfile.TemporaryDirectory(prefix="cj-feedback-")
        self.addCleanup(self.temp.cleanup)
        config = pathlib.Path(self.temp.name) / "codejournal"
        config.mkdir()
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=self.temp.name, CJ_TOKEN="test-token")

    def command(self, *args, stdin=None):
        return subprocess.run([self.binary, "--project", "p", *args], cwd=REPO,
                              env=self.env, input=stdin, text=True, capture_output=True,
                              timeout=3, check=True)

    def test_title_and_note_updates_do_not_read_idle_stdin(self):
        for args in [("plan", "update", "aaaaaaaa", "--title", "New title"),
                     ("plan", "update", "aaaaaaaa", "--note", "CI passed")]:
            process = subprocess.Popen([self.binary, "--project", "p", *args], cwd=REPO,
                                       env=self.env, stdin=subprocess.PIPE,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                self.assertEqual(process.wait(timeout=2), 0)
            finally:
                process.stdin.close()
                process.stdout.close()
                process.stderr.close()
                if process.poll() is None:
                    process.kill()
        self.assertEqual(Handler.calls[0][2]["title"], "New title")
        self.assertEqual(Handler.calls[1][2]["note"], "CI passed")

    def test_piped_body_and_positional_rules(self):
        shown = subprocess.Popen([self.binary, "--project", "p", "plan", "show", "aaaaaaaa", "--body"],
                                 cwd=REPO, env=self.env, stdout=subprocess.PIPE)
        updated = subprocess.run([self.binary, "--project", "p", "plan", "update", "aaaaaaaa",
                                  "--body-file", "-"], cwd=REPO, env=self.env, stdin=shown.stdout,
                                 capture_output=True, timeout=3, check=True)
        shown.stdout.close()
        self.assertEqual(shown.wait(timeout=2), 0)
        self.assertEqual(Handler.calls[-1][2]["body"], "## Steps\n- [ ] ship")
        shown_doc = subprocess.Popen([self.binary, "--project", "p", "doc", "show", "aaaaaaaa", "--body"],
                                     cwd=REPO, env=self.env, stdout=subprocess.PIPE)
        subprocess.run([self.binary, "--project", "p", "doc", "update", "aaaaaaaa", "--body-file", "-"],
                       cwd=REPO, env=self.env, stdin=shown_doc.stdout, capture_output=True,
                       timeout=3, check=True)
        shown_doc.stdout.close()
        self.assertEqual(shown_doc.wait(timeout=2), 0)
        self.assertEqual(Handler.calls[-1][2]["body"], "Documentation body")
        self.command("rules", "set", "- Run make test.")
        self.command("rules", "append", "- Keep commits small.")
        self.assertEqual([call[2]["rules"] for call in Handler.calls[-2:]],
                         ["- Run make test.", "- Keep commits small."])

    def test_shorthand_refs_reach_api(self):
        self.command("add", "--kind", "discovery", "--title", "A fact", "--body", "Body",
                     "--ref", "PR#42", "--ref", "issue#7", "--ref", "path:/docs/")
        self.assertEqual(Handler.calls[-1][2]["refs"], [
            {"kind": "url", "value": "https://github.com/illegalstudio/codejournal/pull/42"},
            {"kind": "issue", "value": "#7"}, {"kind": "path", "value": "docs"},
        ])

    def test_schedules_forwarding_and_framework_hints(self):
        result = self.command("add", "--kind", "gotcha", "--title", "Laravel framework issue",
                              "--body", "Seen locally")
        self.assertNotIn("hint:", result.stdout)
        self.command("task", "add", "--title", "Forwarded work", "--to", "target", "--from-entry", "abcdef12",
                     "--not-before", "tomorrow")
        self.assertEqual(Handler.calls[-1][1], "/api/v1/tenants/demo/projects/target/tasks")
        self.assertEqual(Handler.calls[-1][2]["source_project"], "p")
        self.assertEqual(Handler.calls[-1][2]["not_before"], "tomorrow")
        self.command("plan", "schedule", "aaaaaaaa", "7d")
        self.assertEqual(Handler.calls[-1][2]["not_before"], "7d")


if __name__ == "__main__":
    unittest.main()
