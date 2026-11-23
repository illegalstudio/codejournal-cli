import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
EVENTS = [{"created_at": f"2026-11-{day:02d} 09:00:00+00", "agent": "codex", "kind": "note",
           "note": f"Synthetic note {day}", "status": "open"} for day in range(1, 9)]
TASK = {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "project_slug": "p", "status": "open",
        "priority": "normal", "body_version": "base", "created_at": "2026-11-01 08:00:00+00",
        "agent": "codex", "title": "Synthetic task", "body": "Current body", "events": EVENTS}
ENTRY = {"id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", "project_slug": "p", "kind": "gotcha",
         "status": "active", "created_at": "2026-09-30 08:00:00+00", "title": "Old runbook",
         "body": "Reach the host through the jump box", "scope": "project", "topics": [], "refs": [],
         "usage": {}, "newer_environment": [{"id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
                                             "created_at": "2026-10-26 14:31:15+00",
                                             "title": "The host is reachable directly"}]}
BRIEF = {"project": {"slug": "p", "name": "p"}, "counts": {"active": 1}, "rules": "Synthetic rule",
         "duplicate_checkouts": [{"slug": "older-journal", "host": "synthetic-host",
                                  "path": "/synthetic/checkout", "active_entries": 4, "logs": 17}]}


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        if "/tasks/" in self.path:
            payload = {"task": TASK}
        elif "/entries/" in self.path:
            payload = {"entry": ENTRY}
        else:
            payload = BRIEF
        data = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        self.do_GET()


class November13FeedbackTest(unittest.TestCase):
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
        temp = tempfile.TemporaryDirectory(prefix="cj-november13-")
        self.addCleanup(temp.cleanup)
        base = pathlib.Path(temp.name)
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"),
                        XDG_STATE_HOME=str(base / "state"), XDG_CACHE_HOME=str(base / "cache"),
                        CJ_TOKEN="test-token", CJ_PROJECT="p", CODE_JOURNAL_AUTO_SYNC="off")

    def cli(self, *args):
        result = subprocess.run([self.binary, *args], env=self.env, cwd=ROOT,
                                text=True, capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_task_show_returns_the_latest_events_once_and_history_on_request(self):
        compact = json.loads(self.cli("task", "show", "aaaaaaaa", "--json"))
        self.assertNotIn("events", compact["task"])
        self.assertEqual([event["note"] for event in compact["events"]],
                         [f"Synthetic note {day}" for day in range(4, 9)])
        self.assertEqual((compact["events_total"], compact["events_omitted"]), (8, 3))
        self.assertEqual(compact["history_command"], "cj task show aaaaaaaa --history")
        full = json.loads(self.cli("task", "show", "aaaaaaaa", "--history", "--json"))
        self.assertEqual(len(full["events"]), 8)
        self.assertNotIn("events", full["task"])
        self.assertNotIn("history_command", full)
        text = self.cli("task", "show", "aaaaaaaa")
        self.assertIn("History (latest 5 of 8 events; --history shows all):", text)
        self.assertNotIn("Synthetic note 3", text)
        self.assertIn("Synthetic note 1", self.cli("task", "show", "aaaaaaaa", "--history"))

    def test_task_show_names_the_owning_project_when_changes_need_it(self):
        self.assertNotIn("scope:", self.cli("task", "show", "aaaaaaaa"))
        self.assertNotIn("change_scope", json.loads(self.cli("task", "show", "aaaaaaaa", "--json")))
        self.env["CJ_PROJECT"] = "elsewhere"
        command = "cj task edit aaaaaaaa --project p --base base --append-body TEXT"
        self.assertIn(f"scope:     belongs to p; commands that change it from here need --project p, for example: {command}",
                      self.cli("task", "show", "aaaaaaaa"))
        self.assertEqual(json.loads(self.cli("task", "show", "aaaaaaaa", "--json"))["change_scope"],
                         {"project": "p", "edit_command": command, "readback_command": "cj task show aaaaaaaa"})

    def test_entry_show_lists_newer_environment_facts(self):
        text = self.cli("show", "bbbbbbbb", "--no-track")
        self.assertIn("Newer environment facts sharing rare terms", text)
        self.assertIn("  cccccccc  2026-10-26  The host is reachable directly", text)

    def test_brief_warns_about_a_second_journal_for_the_same_checkout(self):
        text = self.cli("brief")
        self.assertIn("Duplicate journal: synthetic-host:/synthetic/checkout is also registered "
                      "under project older-journal (4 active entries, 17 work logs)", text)
        self.assertIn("cj brief --project older-journal", text)
        self.assertLess(text.index("Duplicate journal"), text.index("Project rules"))


if __name__ == "__main__":
    unittest.main()
