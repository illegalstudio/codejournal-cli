import hashlib
import http.server
import json
import os
import pathlib
import re
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
VERSION = re.search(r'^version = "([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M).group(1)
UPGRADE = f"cj {VERSION} is no longer supported by this server. Install cj 9.0.0 or newer, then retry."
LIMITED = "This workspace's plan allows 60 API requests per minute. Retry in 30 seconds."
REFUSALS = {426: {"message": UPGRADE, "minimum_version": "9.0.0",
                  "error": "client_upgrade_required", "upgrade_required": True},
            429: {"message": LIMITED, "limit": 60, "retry_after": 30}}


class Handler(http.server.BaseHTTPRequestHandler):
    headers_seen = []
    writes = []
    refusal = None

    def log_message(self, *_args):
        pass

    def respond(self, status, payload):
        self.headers_seen.append((self.headers.get("X-Cj-Version"), self.headers.get("User-Agent")))
        if self.refusal:
            status, payload = self.refusal, REFUSALS[self.refusal]
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/api/v1/me":
            self.respond(200, {"tenant": {"slug": "demo"}})
        elif self.path.endswith("/projects"):
            self.respond(200, {"projects": [{"slug": "fixture", "name": "Fixture",
                "active_entries": 0, "has_rules": False}]})
        else:
            self.respond(404, {"error": "missing"})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
        if not self.refusal:
            self.writes.append((self.headers.get("Idempotency-Key"), body))
        self.respond(201, {"entry": {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"}})


class DeferredResponseTest(unittest.TestCase):
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
        Handler.headers_seen.clear()
        Handler.writes.clear()
        Handler.refusal = None
        self.temp = tempfile.TemporaryDirectory(prefix="cj-deferred-")
        self.addCleanup(self.temp.cleanup)
        base = pathlib.Path(self.temp.name)
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"),
                        XDG_STATE_HOME=str(base / "state"), CJ_TOKEN="test-token",
                        CODE_JOURNAL_HOOKS="on", CODE_JOURNAL_HOOK_FLUSH="off")
        self.base = base

    def cli(self, *args, check=True):
        return subprocess.run([self.binary, "--json", "--project", "fixture", *args],
            cwd=self.base, env=self.env, capture_output=True, text=True, timeout=10, check=check)

    def test_requests_carry_the_release(self):
        self.cli("projects")
        self.cli("add", "--kind", "gotcha", "--title", "Versioned write", "--body", "Synthetic")
        for version, agent in Handler.headers_seen:
            self.assertEqual(version, VERSION)
            self.assertTrue(agent.startswith(f"cj/{VERSION} ("))
        self.assertTrue(self.cli("--version").stdout.strip().startswith(f"cj {VERSION} ("))

    def test_login_refusal_has_update_guidance_and_sends_the_release(self):
        Handler.refusal = 426
        result = self.cli("login", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("brew upgrade illegalstudio/tap/codejournal-cli", result.stderr)
        self.assertIn("mise use -g github:illegalstudio/codejournal-cli@9.0.0", result.stderr)
        self.assertEqual(Handler.headers_seen[0][0], VERSION)

    def test_agent_receives_upgrade_notice_with_or_without_cached_brief(self):
        Handler.refusal = 426
        cache = self.base / "state/codejournal/briefs" / (hashlib.sha256(str(self.base).encode()).hexdigest() + "-active-v1.txt")
        for cached in (False, True):
            with self.subTest(cached=cached):
                if cached:
                    cache.parent.mkdir(parents=True, exist_ok=True)
                    cache.write_text("Cached synthetic brief")
                result = subprocess.run([self.binary, "hook", "SessionStart"], cwd=self.base,
                    env=self.env, input=json.dumps({"session_id": "version-test", "cwd": str(self.base)}),
                    capture_output=True, text=True, timeout=10, check=True)
                context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
                self.assertIn("Agents: tell the user an update is required", context)
                self.assertIn("github.com/illegalstudio/codejournal-cli", context)
                self.assertIn("not fresh", context)
                if cached:
                    self.assertIn("Cached synthetic brief", context)
                    self.assertEqual(cache.read_text(), "Cached synthetic brief")

    def test_refused_read_without_cache_shows_the_server_message(self):
        for status, message in ((426, UPGRADE), (429, LIMITED)):
            with self.subTest(status=status):
                Handler.refusal = status
                result = self.cli("projects", check=False)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)

    def test_refused_read_falls_back_to_the_cache_with_a_warning(self):
        online = json.loads(self.cli("projects").stdout)
        for status, message in ((426, UPGRADE), (429, LIMITED)):
            with self.subTest(status=status):
                Handler.refusal = status
                result = self.cli("projects")
                self.assertEqual(json.loads(result.stdout), online)
                self.assertIn(f"warning: {message}", result.stderr)

    def test_refused_write_stays_queued_for_sync(self):
        for status, message in ((426, UPGRADE), (429, LIMITED)):
            with self.subTest(status=status):
                Handler.writes.clear()
                Handler.refusal = status
                result = self.cli("add", "--kind", "gotcha", "--title", "Kept note", "--body", "Test")
                queued = json.loads(result.stdout)
                self.assertTrue(queued["queued"])
                if status == 426:
                    self.assertTrue(queued["upgrade_required"])
                    self.assertEqual(queued["compatibility"]["minimum_version"], "9.0.0")
                self.assertIn(f"warning: {message}", result.stderr)
                self.assertEqual(Handler.writes, [])
                Handler.refusal = None
                self.assertEqual(json.loads(self.cli("sync").stdout)["flushed"], 1)
                self.assertEqual(Handler.writes[0][0], queued["id"])
