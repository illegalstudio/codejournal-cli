import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    get_calls = []
    created = False
    canonical = None

    def log_message(self, *_args):
        pass

    def do_GET(self):
        self.get_calls.append(self.path)
        if "/projects/resolve?" in self.path:
            status, payload = 200, {"project": {"slug": Handler.canonical or "new-project"}}
        elif self.path.endswith("/projects/new-project-2"):
            status, payload = 200, {"project": {"slug": "new-project-2", "name": "New", "counts": {}, "paths": []}}
        elif self.path.endswith("/projects/new-project") and Handler.canonical:
            status, payload = 404, {"message": "Wrong project"}
        elif "/entries?" in self.path:
            status, payload = 200, {"project": Handler.canonical or "new-project", "entries": []}
        else:
            status, payload = 200, {"user": {}}
        encoded = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.calls.append((self.path, body))
        if self.path.endswith("/projects"):
            Handler.created = True
            status, data = 201, {"project": {"slug": Handler.canonical or body["slug"]}}
        elif Handler.canonical and self.path.endswith("/entries") and f"/{Handler.canonical}/" not in self.path:
            status, data = 404, {"message": "Project not found"}
        elif self.path.endswith("/entries") and not self.created:
            status, data = 404, {"message": "Project not found"}
        elif self.path.endswith("/entries"):
            status, data = 201, {"entry": {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"}}
        else:
            status, data = 201, {"checkout": body}
        encoded = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)


class AutoProjectTest(unittest.TestCase):
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
        Handler.get_calls.clear()
        Handler.created = False
        Handler.canonical = None
        self.temp = tempfile.TemporaryDirectory(prefix="cj-auto-project-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.repo = self.base / "new-project"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        config = self.base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(config.parent),
                        XDG_STATE_HOME=str(self.base / "state"), CJ_TOKEN="test-token")

    def run_add(self, *extra):
        return subprocess.run([self.binary, *extra, "add", "--kind", "discovery", "--title", "First entry",
                               "--body", "Synthetic body"], cwd=self.repo, env=self.env,
                              capture_output=True, text=True, timeout=8)

    def test_first_write_creates_repository_project(self):
        result = self.run_add()
        self.assertEqual(result.returncode, 0, result.stderr)
        paths = [path for path, _ in Handler.calls]
        self.assertEqual(paths, [
            "/api/v1/tenants/demo/projects",
            "/api/v1/tenants/demo/projects/new-project/entries",
        ])
        self.assertEqual(Handler.calls[0][1]["path"], str(self.repo))
        self.assertEqual(Handler.calls[0][1]["kind"], "main")

    def test_offline_first_write_queues_project_before_entry(self):
        result = self.run_add("--offline")
        self.assertEqual(result.returncode, 0, result.stderr)
        queued = sorted((self.base / "state" / "codejournal" / "requests").glob("*.json"))
        paths = [json.loads(path.read_text())["path"] for path in queued]
        self.assertEqual(paths, [
            "/api/v1/tenants/demo/projects",
            "/api/v1/tenants/demo/projects/new-project/entries",
        ])
        self.assertEqual(Handler.calls, [])

    def test_offline_project_init_queues_its_checkout(self):
        result = subprocess.run([self.binary, "--offline", "project", "init"], cwd=self.repo,
                                env=self.env, capture_output=True, text=True, timeout=8)
        self.assertEqual(result.returncode, 0, result.stderr)
        queued = sorted((self.base / "state" / "codejournal" / "requests").glob("*.json"))
        self.assertEqual([json.loads(path.read_text())["path"] for path in queued], [
            "/api/v1/tenants/demo/projects",
        ])
        self.assertEqual(json.loads(queued[0].read_text())["body"]["path"], str(self.repo))

    def test_collision_uses_canonical_slug_for_first_write(self):
        Handler.canonical = "new-project-2"
        result = self.run_add()
        self.assertEqual(result.returncode, 0, result.stderr)
        again = self.run_add()
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertEqual([path for path, _ in Handler.calls], [
            "/api/v1/tenants/demo/projects",
            "/api/v1/tenants/demo/projects/new-project-2/entries",
            "/api/v1/tenants/demo/projects/new-project-2/entries",
        ])

    def test_offline_collision_rewrites_queued_paths_on_replay(self):
        self.assertEqual(self.run_add("--offline").returncode, 0)
        Handler.canonical = "new-project-2"
        synced = subprocess.run([self.binary, "sync"], cwd=self.repo, env=self.env,
                                capture_output=True, text=True, timeout=8)
        self.assertEqual(synced.returncode, 0, synced.stderr)
        self.assertEqual([path for path, _ in Handler.calls], [
            "/api/v1/tenants/demo/projects",
            "/api/v1/tenants/demo/projects/new-project-2/entries",
        ])

    def test_first_read_resolves_existing_project_without_creating_it(self):
        Handler.canonical = "new-project-2"
        result = subprocess.run([self.binary, "--json", "project", "show"], cwd=self.repo,
                                env=self.env, capture_output=True, text=True, timeout=8)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["project"]["slug"], "new-project-2")
        self.assertTrue(any("/projects/resolve?" in path for path in Handler.get_calls))
        self.assertEqual(Handler.calls, [])
        cached = subprocess.run([self.binary, "--offline", "--json", "project", "show"],
                                cwd=self.repo, env=self.env, capture_output=True, text=True, timeout=8)
        self.assertEqual(cached.returncode, 0, cached.stderr)

    def test_path_add_uses_resolved_project_slug(self):
        Handler.canonical = "new-project-2"
        result = subprocess.run([self.binary, "project", "path-add", str(self.repo)],
                                cwd=self.repo, env=self.env, capture_output=True, text=True, timeout=8)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([path for path, _ in Handler.calls], [
            "/api/v1/tenants/demo/projects/new-project-2/paths",
        ])

    def test_search_and_open_use_resolved_project_slug(self):
        Handler.canonical = "new-project-2"
        searched = subprocess.run([self.binary, "--json", "search", "sample"], cwd=self.repo,
                                  env=self.env, capture_output=True, text=True, timeout=8)
        self.assertEqual(searched.returncode, 0, searched.stderr)
        self.assertTrue(any("/entries?" in path and "project=new-project-2" in path
                            for path in Handler.get_calls))
        opened = subprocess.run([self.binary, "--json", "open"], cwd=self.repo,
                                env=self.env, capture_output=True, text=True, timeout=8)
        self.assertEqual(opened.returncode, 0, opened.stderr)
        self.assertTrue(json.loads(opened.stdout)["url"].endswith("#/p/new-project-2"))


if __name__ == "__main__":
    unittest.main()
