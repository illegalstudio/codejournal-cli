import http.server
from cli_usability_server import Handler
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest
import urllib.parse

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
        for kind in ["doc", "plan"]:
            current = self.cli(kind, "show", "aaaaaaaa", "--json")
            self.assertEqual(current.returncode, 0, current.stderr)
            data = json.loads(current.stdout)
            self.assertEqual(data["revisions"], [])
            self.assertEqual(data["revision_count"], 60)
            self.assertEqual(data["current_revision"], 60)
            self.assertEqual(data[kind]["body"], "Current body")
            self.assertNotIn("Previous body", current.stdout)
            self.assertTrue(Handler.paths[-1].endswith("?history=0"))
            explicit = self.cli(kind, "show", "aaaaaaaa", "--json", "--current-only")
            self.assertEqual(json.loads(explicit.stdout), data)
            human = self.cli(kind, "show", "aaaaaaaa")
            self.assertIn("revisions: 60 (current 60)", human.stdout)

    def test_one_selected_revision_and_complete_history_are_explicit(self):
        for kind in ["doc", "plan"]:
            selected = self.cli(kind, "show", "aaaaaaaa", "--revision", "59", "--json")
            self.assertEqual(selected.returncode, 0, selected.stderr)
            data = json.loads(selected.stdout)
            self.assertEqual(data[kind]["revision"], 59)
            self.assertEqual(data[kind]["body"], "Previous body 59")
            self.assertEqual(data["revisions"], [])
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(Handler.paths[-1]).query)
            self.assertEqual(query, {"revision": ["59"], "history": ["0"]})
            all_versions = self.cli(kind, "show", "aaaaaaaa", "--all-revisions", "--json")
            self.assertEqual(all_versions.returncode, 0, all_versions.stderr)
            self.assertEqual(len(json.loads(all_versions.stdout)["revisions"]), 60)
            self.assertIn("Previous body 1", all_versions.stdout)
            self.assertTrue(Handler.paths[-1].endswith("?history=1"))
            human = self.cli(kind, "show", "aaaaaaaa", "--all-revisions")
            self.assertIn("Previous body 59", human.stdout)
            self.assertIn("Version 1", human.stdout)
            self.assertNotEqual(self.cli(kind, "show", "aaaaaaaa", "--revision", "61").returncode, 0)
            help_text = self.cli(kind, "show", "--help").stdout
            self.assertIn("--all-revisions", help_text)
            self.assertIn("numbered revision", help_text)

    def test_plan_and_doc_reads_skip_history_or_request_a_metadata_page(self):
        for kind in ["plan", "doc"]:
            for options in [[], ["--body"], ["--body", "--json"]]:
                result = self.cli(kind, "show", "aaaaaaaa", *options)
                self.assertEqual(result.returncode, 0, result.stderr)
                query = urllib.parse.parse_qs(urllib.parse.urlsplit(Handler.paths[-1]).query)
                self.assertEqual(query, {"history": ["0"]})
                self.assertIn("Current body", result.stdout)
            result = self.cli(kind, "show", "aaaaaaaa", "--history")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("--before-revision 41", result.stdout)
            self.assertNotIn("Previous body", result.stdout)
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(Handler.paths[-1]).query)
            self.assertEqual(query, {"history_content": ["0"]})
            result = self.cli(kind, "show", "aaaaaaaa", "--history", "--before-revision", "20")
            self.assertEqual(result.returncode, 0, result.stderr)
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(Handler.paths[-1]).query)
            self.assertEqual(query, {"history_content": ["0"], "history_before": ["20"]})
            for options in [["--before-revision", "20"], ["--history", "--before-revision", "0"],
                            ["--history", "--before-revision", "2147483648"],
                            ["--revision", "0"], ["--revision", "2147483648"],
                            ["--revision", "1", "--history"], ["--all-revisions", "--history"],
                            ["--all-revisions", "--body"], ["--all-revisions", "--revision", "1"]]:
                paths = list(Handler.paths)
                result = self.cli(kind, "show", "aaaaaaaa", *options)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(paths, Handler.paths)

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
