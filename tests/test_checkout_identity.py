import http.server
import json
import os
import pathlib
import shutil
import socket
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    checkouts = []
    remote = None

    def log_message(self, *_args):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.calls.append((self.path, body))
        data = json.dumps({"project": body} if self.path.endswith("/projects")
                          else {"checkout": body}).encode()
        self.send_response(201)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        data = json.dumps({"checkouts": self.checkouts} if self.path.endswith("/checkouts")
                          else {"projects": []} if self.path.endswith("/projects")
                          else {"project": {"remote_url": self.remote}}).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


class CheckoutIdentityTest(unittest.TestCase):
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
        Handler.checkouts.clear()
        Handler.remote = None
        self.temp = tempfile.TemporaryDirectory(prefix="cj-checkout-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.source = self.base / "source"
        self.source.mkdir()
        self.git(self.source, "init", "-b", "main")
        self.git(self.source, "config", "user.email", "test@example.test")
        self.git(self.source, "config", "user.name", "Test")
        (self.source / "README.md").write_text("Synthetic checkout\n")
        self.git(self.source, "add", "README.md")
        self.git(self.source, "commit", "-m", "initial")
        config = self.base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(config.parent),
                        XDG_STATE_HOME=str(self.base / "state"), CJ_TOKEN="test-token")

    def git(self, cwd, *args):
        subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)

    def init(self, cwd):
        subprocess.run([self.binary, "project", "init"], cwd=cwd, env=self.env,
                       check=True, capture_output=True, text=True, timeout=5)
        project = next(body for path, body in Handler.calls if path.endswith("/projects"))
        return project, project

    def test_copy_on_write_snapshot_reuses_source_project(self):
        snapshot = self.base / "snapshot"
        shutil.copytree(self.source, snapshot)
        self.git(snapshot, "remote", "add", "main", str(self.source))
        self.git(snapshot, "checkout", "-b", "snap/branch")
        project, checkout = self.init(snapshot)
        self.assertEqual(project["slug"], "source")
        self.assertEqual((checkout["kind"], checkout["main_path"]), ("cow", str(self.source)))
        self.assertEqual(checkout["branch"], "snap/branch")

    def test_foreign_origin_does_not_take_source_identity(self):
        self.git(self.source, "remote", "add", "origin", "git@github.com:acme/source.git")
        foreign = self.base / "foreign"
        shutil.copytree(self.source, foreign)
        self.git(foreign, "remote", "set-url", "origin", "git@github.com:other/foreign.git")
        self.git(foreign, "remote", "add", "main", str(self.source))
        project, checkout = self.init(foreign)
        self.assertEqual(project["slug"], "other-foreign")
        self.assertEqual(checkout["kind"], "main")

    def test_snapshot_chain_resolves_to_first_checkout(self):
        snapshot = self.base / "snapshot"
        shutil.copytree(self.source, snapshot)
        self.git(snapshot, "remote", "add", "main", str(self.source))
        chained = self.base / "chained"
        shutil.copytree(snapshot, chained)
        self.git(chained, "remote", "set-url", "main", str(snapshot))
        project, checkout = self.init(chained)
        self.assertEqual(project["slug"], "source")
        self.assertEqual((checkout["kind"], checkout["main_path"]), ("cow", str(self.source)))

    def test_orphan_snapshot_keeps_its_own_identity(self):
        self.git(self.source, "remote", "add", "main", str(self.base / "deleted"))
        project, checkout = self.init(self.source)
        self.assertEqual(project["slug"], "source")
        self.assertEqual((checkout["kind"], checkout["main_path"]), ("cow", None))

    def test_linked_worktree_resolves_to_main_checkout(self):
        worktree = self.base / "worktree"
        self.git(self.source, "worktree", "add", "-b", "feature", str(worktree))
        project, checkout = self.init(worktree)
        self.assertEqual(project["slug"], "source")
        self.assertEqual((checkout["kind"], checkout["main_path"]), ("worktree", str(self.source)))

    def test_normalize_reclassifies_legacy_worktree_record(self):
        worktree = self.base / "worktree"
        self.git(self.source, "worktree", "add", "-b", "feature", str(worktree))
        Handler.checkouts.append({"project_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                                  "project_slug": "source", "remote_url": None,
                                  "host": socket.gethostname(), "path": str(worktree),
                                  "kind": "main", "branch": None, "main_path": None})
        result = subprocess.run([self.binary, "--json", "normalize", "--dry-run"],
                                cwd=worktree, env=self.env, capture_output=True, text=True,
                                timeout=5, check=True)
        report = json.loads(result.stdout)
        self.assertEqual(report["updates"][0]["new_kind"], "worktree")
        self.assertEqual(report["updates"][0]["new_main_path"], str(self.source))

    def test_normalize_merges_remote_free_worktree_into_main_owner(self):
        worktree = self.base / "worktree"
        self.git(self.source, "worktree", "add", "-b", "feature", str(worktree))
        host = socket.gethostname()
        Handler.checkouts.extend([
            {"project_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
             "project_slug": "source", "remote_url": None, "host": host,
             "path": str(self.source), "kind": "main", "branch": None, "main_path": None},
            {"project_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
             "project_slug": "legacy-worktree", "remote_url": None, "host": host,
             "path": str(worktree), "kind": "main", "branch": None, "main_path": None},
        ])
        result = subprocess.run([self.binary, "--json", "normalize", "--dry-run"],
                                cwd=worktree, env=self.env, capture_output=True, text=True,
                                timeout=5, check=True)
        report = json.loads(result.stdout)
        self.assertEqual(report["merges"], [{"from": "legacy-worktree", "into": "source"}])

    def test_path_add_rejects_a_different_origin(self):
        self.git(self.source, "remote", "add", "origin", "git@github.com:other/foreign.git")
        Handler.remote = "github.com/acme/source"
        result = subprocess.run([self.binary, "--project", "source", "project", "path-add", str(self.source)],
                                cwd=self.source, env=self.env, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("update the project remote first", result.stderr)
        self.assertFalse(any(path.endswith("/paths") for path, _ in Handler.calls))


if __name__ == "__main__":
    unittest.main()
