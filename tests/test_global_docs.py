import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest
from urllib.parse import parse_qs, urlsplit


ROOT = pathlib.Path(__file__).resolve().parents[1]
DOC = {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "title": "Shared runbook",
       "scope": "global", "project_id": None, "project_slug": None, "status": "current",
       "body": "Recovery", "revision": 1, "updated_at": "2026-09-30T10:00:00Z"}


class Handler(http.server.BaseHTTPRequestHandler):
    requests = []

    def log_message(self, *_args):
        pass

    def respond(self, payload, status=200):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self.requests.append(("GET", self.path, None, None))
        if "/docs/" in self.path:
            self.respond({"doc": DOC, "revisions": []})
        elif "/docs?" in self.path:
            local = "/projects/fixture/" in self.path
            docs = [{**DOC, "scope": "project", "project_slug": "fixture", "title": "Project recovery"}] if local else [DOC]
            if local and parse_qs(urlsplit(self.path).query).get("include_global") == ["1"]:
                docs.append(DOC)
            self.respond({"docs": docs})
        else:
            self.respond({})

    def write_request(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.requests.append((self.command, self.path, body, self.headers.get("Idempotency-Key")))
        if self.path.endswith("/brief"):
            self.respond({"project": {"slug": "fixture"}, "rules": "Test rules", "counts": {},
                "global_docs": [{**DOC, "title": "Unmatched shared"}],
                "focus": {"changes": {"files": ["src/app.php"]}, "entries": [],
                    "docs": [{**DOC, "title": "Matched shared"}]}})
        else:
            self.respond({"doc": DOC, "revision": 2}, 201 if self.command == "POST" else 200)

    do_POST = write_request
    do_PATCH = write_request


class GlobalDocsTest(unittest.TestCase):
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
        Handler.requests.clear()
        self.temp = tempfile.TemporaryDirectory(prefix="cj-global-docs-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        config = self.base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(self.base / "config"),
                        XDG_STATE_HOME=str(self.base / "state"), CJ_TOKEN="test-token")
        self.env.pop("CJ_PROJECT", None)

    def cli(self, *args, check=True):
        return subprocess.run([self.binary, *args], cwd=self.base, env=self.env,
            capture_output=True, text=True, timeout=10, check=check)

    def test_global_creation_flags_and_project_compatibility(self):
        self.assertIn("Created doc aaaaaaaa", self.cli("doc", "create", "--global",
            "--title", "Shared runbook", "--body", "Recovery").stdout)
        self.assertEqual(Handler.requests[-1][1], "/api/v1/tenants/demo/docs")
        self.cli("--project", "fixture", "doc", "create", "--title", "Local", "--body", "Body")
        self.assertEqual(Handler.requests[-1][1], "/api/v1/tenants/demo/projects/fixture/docs")
        conflict = self.cli("--project", "fixture", "doc", "create", "--global",
            "--title", "Bad", "--body", "Body", check=False)
        self.assertNotEqual(conflict.returncode, 0)
        self.assertNotEqual(self.cli("plan", "create", "--global", "--title", "Bad",
            "--body", "Body", check=False).returncode, 0)
        self.assertNotEqual(self.cli("plan", "move", "aaaaaaaa", "--to", "@global", check=False).returncode, 0)

    def test_lists_and_detail_handle_global_scope_and_cached_null_project(self):
        self.assertIn("GLOBAL", self.cli("doc", "list", "--global").stdout)
        self.assertIn("GLOBAL", self.cli("doc", "list", "--all-projects").stdout)
        self.assertIn("GLOBAL", self.cli("--project", "fixture", "doc", "list").stdout)
        self.assertNotIn("GLOBAL", self.cli("--project", "fixture", "doc", "list", "--local").stdout)
        online = json.loads(self.cli("--json", "doc", "list", "--global").stdout)
        offline = json.loads(self.cli("--json", "--offline", "doc", "list", "--global").stdout)
        self.assertTrue(offline.pop("cached"))
        self.assertEqual(online, offline)
        self.assertIn("project:   GLOBAL", self.cli("doc", "show", "aaaaaaaa").stdout)

    def test_default_search_includes_shared_docs_and_local_is_explicit(self):
        result = self.cli("--project", "fixture", "doc", "list", "--grep", "recovery")
        self.assertIn("Project recovery", result.stdout)
        self.assertIn("GLOBAL", result.stdout)
        query = parse_qs(urlsplit(Handler.requests[-1][1]).query)
        self.assertEqual(query["include_global"], ["1"])
        self.assertEqual(query["grep"], ["recovery"])
        self.cli("--project", "fixture", "doc", "list", "--local", "--grep", "recovery")
        self.assertNotIn("include_global", parse_qs(urlsplit(Handler.requests[-1][1]).query))
        for flag in ["--global", "--all-projects"]:
            self.assertNotEqual(self.cli("doc", "list", "--local", flag, check=False).returncode, 0)
        self.cli("--project", "fixture", "plan", "list")
        self.assertNotIn("include_global", parse_qs(urlsplit(Handler.requests[-1][1]).query))

    def test_offline_global_writes_keep_route_target_and_idempotency_key(self):
        queued = json.loads(self.cli("--json", "--offline", "doc", "create", "--global",
            "--title", "Shared", "--body", "Body").stdout)
        self.assertTrue(queued["queued"])
        self.cli("sync")
        self.assertEqual(Handler.requests[-1][1], "/api/v1/tenants/demo/docs")
        self.assertEqual(Handler.requests[-1][3], queued["id"])
        self.cli("doc", "show", "aaaaaaaa", "--current-only")
        self.cli("--offline", "doc", "move", "aaaaaaaa", "--to", "@global")
        self.cli("sync")
        self.assertEqual(Handler.requests[-1][2]["to"], "@global")

    def test_full_and_compact_briefs_bound_shared_context(self):
        full = self.cli("--project", "fixture", "brief").stdout
        self.assertIn("Global docs", full)
        self.assertIn("Unmatched shared", full)
        compact = self.cli("--project", "fixture", "brief", "--compact").stdout
        self.assertIn("Matched shared", compact)
        self.assertNotIn("Unmatched shared", compact)
