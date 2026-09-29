import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
LOCKED = ("This project is locked on the Free plan, which reads one active project. Writes are kept. "
          "Switch the active project in the dashboard settings or upgrade to Pro.")


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def respond(self, status, payload):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path.endswith("/projects"):
            self.respond(200, {"projects": [
                {"slug": "active", "name": "Active", "active_entries": 1, "has_rules": True, "locked": False},
                {"slug": "fixture", "name": "Fixture", "active_entries": 0, "has_rules": False, "locked": True}]})
        else:
            # Every other read of the locked project, whatever route the command uses.
            self.respond(402, {"locked": True, "message": LOCKED})

    def do_POST(self):
        self.rfile.read(int(self.headers["Content-Length"]))
        if self.path.endswith("/brief"):
            self.respond(200, {"project": {"slug": "fixture", "remote_url": None}, "locked": True,
                "rules": "", "counts": {}, "recent": [], "entries": [],
                "notices": [LOCKED + " The active project is active."]})
        else:
            self.respond(404, {"message": "missing"})


class FreePlanTest(unittest.TestCase):
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
        self.temp = tempfile.TemporaryDirectory(prefix="cj-free-")
        self.addCleanup(self.temp.cleanup)
        base = pathlib.Path(self.temp.name)
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"),
                        XDG_STATE_HOME=str(base / "state"), CJ_TOKEN="test-token")
        self.base = base

    def cli(self, *args, check=True):
        return subprocess.run([self.binary, "--project", "fixture", *args], cwd=self.base, env=self.env,
            capture_output=True, text=True, timeout=10, check=check)

    def test_locked_brief_shows_only_the_plan_notice(self):
        brief = self.cli("brief").stdout
        self.assertIn(f"Plan: {LOCKED} The active project is active.", brief)
        self.assertNotIn("Project rules", brief)

    def test_locked_reads_print_the_server_message(self):
        result = self.cli("search", "anything", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(LOCKED, result.stderr)
        self.assertNotIn("API returned", result.stderr)

    def test_projects_mark_locked_ones(self):
        lines = self.cli("projects").stdout.splitlines()
        self.assertTrue(any(line.strip().startswith("fixture") and "(locked on Free)" in line for line in lines))
        self.assertFalse(any(line.strip().startswith("active") and "locked" in line for line in lines))
