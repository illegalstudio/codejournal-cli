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
        elif "/search?" in self.path:
            self.respond({"entries": [ENTRY]})
        else:
            self.respond({"entries": []})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.observations.append(body["observations"][0])
        observation = body["observations"][0]
        elsewhere = [{"path": "src/helper.rs", "where": "branch feat/helper"}]
        check = {"missing": [], "elsewhere": elsewhere} if "feat/helper" in observation["branches"] else {
            "missing": ["src/helper.rs"], "elsewhere": []
        }
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
        return json.loads(subprocess.run([self.binary, *args], cwd=self.repo, env=self.env,
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


if __name__ == "__main__":
    unittest.main()
