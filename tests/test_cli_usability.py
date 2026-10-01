import http.server
from cli_usability_server import Handler
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]




class CliUsabilityTest(unittest.TestCase):
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
        Handler.paths.clear()
        temp = tempfile.TemporaryDirectory(prefix="cj-usability-")
        self.addCleanup(temp.cleanup)
        base = pathlib.Path(temp.name)
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"),
                        XDG_STATE_HOME=str(base / "state"), XDG_CACHE_HOME=str(base / "cache"),
                        CJ_TOKEN="test-token", CJ_PROJECT="p")

    def cli(self, *args):
        return subprocess.run([self.binary, *args], env=self.env, cwd=ROOT,
                              text=True, capture_output=True, timeout=5)

    def test_invalid_values_fail_before_any_api_request(self):
        for args in [("plan", "status", "aaaaaaaa", "completed"),
                     ("doc", "status", "aaaaaaaa", "archived"),
                     ("add", "--kind", "fact", "--title", "Test"),
                     ("feedback", "add", "--category", "ux", "--title", "Test")]:
            result = self.cli(*args)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("possible values", result.stderr)
        result = self.cli("log", "list", "--since", "30 days ago")
        self.assertIn("30d", result.stderr)
        self.assertEqual(Handler.paths, [])

    def test_help_names_values_and_replacement_semantics(self):
        add_help = self.cli("add", "--help").stdout
        for kind in ["discovery", "architecture", "decision", "gotcha", "howto", "environment", "question"]:
            self.assertIn(kind, add_help)
        self.assertIn("abandoned", self.cli("plan", "status", "--help").stdout)
        self.assertIn("outdated", self.cli("doc", "status", "--help").stdout)
        self.assertNotIn("abandoned", self.cli("doc", "status", "--help").stdout)
        self.assertIn("@global", self.cli("doc", "move", "--help").stdout)
        self.assertIn("Replace the entire ref list", self.cli("doc", "update", "--help").stdout)

    def test_current_only_does_not_download_revision_bodies(self):
        old = json.loads(self.cli("doc", "show", "aaaaaaaa", "--json").stdout)
        self.assertEqual(len(old["revisions"]), 1)
        current = self.cli("doc", "show", "aaaaaaaa", "--json", "--current-only")
        self.assertEqual(current.returncode, 0, current.stderr)
        self.assertEqual(json.loads(current.stdout)["revisions"], [])
        self.assertTrue(Handler.paths[-1].endswith("?history=0"))
        self.assertIn("Current body", current.stdout)

    def test_log_show_prints_body_refs_and_structured_json(self):
        shown = self.cli("log", "show", "aaaaaaaa")
        self.assertIn("commit:abcdef123", shown.stdout)
        self.assertIn("Current body", shown.stdout)
        self.assertEqual(self.cli("log", "show", "aaaaaaaa", "--body").stdout.strip(), "Current body")
        self.assertEqual(json.loads(self.cli("log", "show", "aaaaaaaa", "--json").stdout)["log"]["title"], "Current")

    def test_api_error_never_prints_laravel_debug_details(self):
        result = self.cli("log", "show", "bbbbbbbb")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Log not found", result.stderr)
        self.assertLess(len(result.stderr), 150)
        for text in ["PrivateClass", "/private", "trace", "secret", "hidden"]:
            self.assertNotIn(text, result.stderr)

    def test_missing_entry_and_global_target_have_actionable_hints(self):
        result = self.cli("show", "aaaaaaaa")
        self.assertIn("cj log show ID", result.stderr)
        result = self.cli("doc", "move", "aaaaaaaa", "--to", "global")
        self.assertIn("--to @global", result.stderr)
        self.assertNotIn("trace", result.stderr)
        result = self.cli("brief")
        self.assertIn("cj project init", result.stderr)
