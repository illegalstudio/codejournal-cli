import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
ENTRY_ID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
ENTRY = {"id": ENTRY_ID, "title": "Branch helper", "refs": [
    {"kind": "path", "value": "src/helper.rs"},
    {"kind": "branch", "value": "feat/helper"},
]}


class Handler(http.server.BaseHTTPRequestHandler):
    observations = []
    maintenance = []
    briefs = []

    def log_message(self, *_args):
        pass

    def respond(self, payload):
        data = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if "/garden?page=1" in self.path:
            self.respond({"entries": [ENTRY], "next_page": 2})
        elif "/garden?page=2" in self.path:
            self.respond({"entries": [{"id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                                       "title": "No paths", "refs": []}], "next_page": None})
        elif "/entries?" in self.path:
            self.respond({"entries": [ENTRY]})
        elif self.path.endswith("/topics"):
            self.respond({"topics": []})
        elif self.path.endswith("/garden/maintenance"):
            self.respond({"secrets": [], "reported_wrong": []})
        elif "/export?" in self.path:
            self.respond({"records": []})
        elif "/brief?" in self.path:
            self.respond({"project": {"slug": "repo", "remote_url": None}, "rules": "Test",
                          "counts": {}, "entries": [], "recent": []})
        else:
            self.respond({"entries": []})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path.endswith("/projects"):
            self.respond({"project": {"slug": body["slug"]}})
            return
        if self.path.endswith("/paths"):
            self.respond({"checkout": body})
            return
        if self.path.endswith("/brief"):
            self.briefs.append(body)
            self.respond({"project": {"slug": "repo", "remote_url": None}, "rules": "Test",
                          "counts": {}, "entries": [], "recent": [],
                          "focus": {"changes": body, "entries": [ENTRY], "docs": []}})
            return
        if self.path.endswith("/brief/focus"):
            self.respond({"focus": {"changes": body, "entries": [ENTRY], "docs": []}})
            return
        if self.path.endswith("/garden/maintenance"):
            self.maintenance.append(body)
            self.respond({"applied": []})
            return
        self.observations.append(body["observations"][0])
        observation = body["observations"][0]
        elsewhere = [{"path": "src/helper.rs", "where": "branch feat/helper"}]
        if observation["present"]:
            check = {"missing": [], "elsewhere": []}
        elif "feat/helper" in observation["branches"]:
            check = {"missing": [], "elsewhere": elsewhere}
        else:
            check = {"missing": [observation["path"]], "elsewhere": []}
        self.respond({"checks": {ENTRY_ID: check}})


class BranchFeedbackTest(unittest.TestCase):
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
        Handler.observations.clear()
        Handler.maintenance.clear()
        Handler.briefs.clear()
        self.temp = tempfile.TemporaryDirectory(prefix="cj-branch-")
        self.addCleanup(self.temp.cleanup)
        base = pathlib.Path(self.temp.name)
        self.repo = base / "repo"
        self.repo.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.test")
        self.git("config", "user.name", "Test")
        (self.repo / "README.md").write_text("initial\n")
        self.git("add", "README.md")
        self.git("commit", "-m", "initial")
        self.git("checkout", "-b", "feat/helper")
        (self.repo / "src").mkdir()
        (self.repo / "src" / "helper.rs").write_text("fn helper() {}\n")
        self.git("add", "src/helper.rs")
        self.git("commit", "-m", "helper")
        self.git("checkout", "main")
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(config.parent), CJ_TOKEN="test-token")

    def git(self, *args):
        subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True)

    def cli(self, *args):
        return json.loads(subprocess.run([self.binary, "--json", *args], cwd=self.repo, env=self.env,
                                         capture_output=True, text=True, timeout=5, check=True).stdout)

    def test_branch_path_is_elsewhere_in_search_and_garden(self):
        found = self.cli("search", "branch")
        self.assertEqual(found["entries"][0]["staleness"]["missing"], [])
        self.assertEqual(found["entries"][0]["staleness"]["elsewhere"][0]["where"], "branch feat/helper")
        report = self.cli("garden", "--dry-run")
        self.assertEqual(report["stale_entries"], [])
        self.assertEqual(report["other_branch_entries"][0]["id"], ENTRY_ID)
        self.assertFalse(Handler.observations[0]["present"])
        self.assertEqual(Handler.observations[0]["branches"], ["feat/helper"])

    def test_brief_highlights_changed_paths(self):
        (self.repo / "untracked.txt").write_text("new\n")
        report = self.cli("brief")
        self.assertIn("untracked.txt", report["focus"]["changes"]["files"])
        self.assertEqual(report["focus"]["entries"][0]["id"], ENTRY_ID)
        self.assertEqual(len(Handler.briefs), 1)

    def test_garden_applies_safe_fixes_through_api(self):
        report = self.cli("garden")
        self.assertFalse(report["dry_run"])
        self.assertEqual(Handler.maintenance, [{"topic_groups": []}])

    def test_five_commits_to_a_present_path_make_entry_stale(self):
        prior = ENTRY["refs"]
        ENTRY.update({"refs": [{"kind": "path", "value": "README.md"}], "created_at": "2020-01-01T00:00:00Z"})
        try:
            for number in range(5):
                (self.repo / "README.md").write_text(f"change {number}\n")
                self.git("add", "README.md")
                self.git("commit", "-m", f"change {number}")
            report = self.cli("garden", "--dry-run")
            self.assertEqual(report["stale_entries"][0]["id"], ENTRY_ID)
            self.assertGreaterEqual(report["stale_entries"][0]["staleness"]["changes"], 5)
        finally:
            ENTRY["refs"] = prior
            ENTRY.pop("created_at", None)


if __name__ == "__main__":
    unittest.main()
