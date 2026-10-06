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
RESOURCE = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    slug = "fixture"
    failed = False
    missing = False
    receipts = {}
    rules = "Preserve known rules"

    def log_message(self, *_):
        pass

    def reply(self, status, value):
        data = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        path = urllib.parse.urlparse(self.path).path
        self.calls.append(("GET", self.path, None))
        if "/requests/" in path:
            key = path.rsplit("/", 1)[1]
            return self.reply(200, self.receipts[key]) if key in self.receipts else self.reply(404, {"message": "Request receipt not found"})
        if self.failed:
            return self.reply(500, {"message": "Server failure"})
        if self.missing:
            return self.reply(404, {"message": "Project not found"})
        self.reply(200, {"project": {"slug": self.slug}, "rules": self.rules, "entries": []})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.calls.append(("POST", self.path, body))
        if self.failed:
            return self.reply(500, {"message": "Server failure"})
        if self.path.endswith("/brief"):
            return self.reply(200, {"project": {"slug": self.slug}, "rules": self.rules, "docs": [{"id": RESOURCE, "status": "current", "title": "Remote guide"}]})
        if self.path.endswith("/projects"):
            return self.reply(201, {"project": {"slug": self.slug}})
        collection = self.path.rsplit("/", 1)[1]
        kind = "entry" if collection == "entries" else collection.removesuffix("s")
        key = self.headers.get("Idempotency-Key")
        self.receipts[key] = {"request_id": key, "resource_type": collection, "resource_id": RESOURCE}
        self.reply(201, {kind: {"id": RESOURCE, "title": body.get("title"), "status": "current"}})

    def do_PATCH(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.calls.append(("PATCH", self.path, body))
        if "slug" in body:
            Handler.slug = body["slug"]
        self.reply(200, {"project": {"slug": self.slug}, "plan": {"id": RESOURCE, "revision": 2}})


class AuditCase(unittest.TestCase):
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
        cls.thread.join(2)

    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="cj-feedback-audit-")
        self.addCleanup(temp.cleanup)
        self.base = pathlib.Path(temp.name)
        self.repo = self.base / "fixture"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        self.config = self.base / "config/codejournal/config.json"
        self.config.parent.mkdir(parents=True)
        self.config.write_text(json.dumps({"server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, HOME=str(self.base), XDG_CONFIG_HOME=str(self.base / "config"),
                        XDG_STATE_HOME=str(self.base / "state"), XDG_CACHE_HOME=str(self.base / "cache"),
                        CJ_TOKEN="synthetic", CODE_JOURNAL_HOOK_FLUSH="off")
        for key in ["CJ_PROJECT", "CJ_SESSION_ID", "CLAUDE_CODE_SESSION_ID", "CODEX_THREAD_ID"]:
            self.env.pop(key, None)
        Handler.calls.clear()
        Handler.receipts.clear()
        Handler.slug, Handler.failed, Handler.missing, Handler.rules = "fixture", False, False, "Preserve known rules"

    def cli(self, *args, stdin=None):
        return subprocess.run([self.binary, *args], cwd=self.repo, env=self.env,
                              input=stdin, capture_output=True, text=True, timeout=15)
