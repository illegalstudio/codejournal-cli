import json
import urllib.parse
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler


STATES = {"docs": ["current", "draft", "outdated"], "plans": ["active", "draft", "done", "abandoned"],
          "tasks": ["open", "in_progress", "done", "dismissed"], "feedback": ["open", "done", "dismissed"],
          "entries": ["active", "obsolete", "superseded"], "logs": ["done", "in_progress", "blocked"],
          "watches": ["starting", "running", "finished", "cancelled", "lost"]}


class DiscoveryDefaultsTest(AuditCase):
    @staticmethod
    def get_response(handler):
        Handler.calls.append(("GET", handler.path, None))
        kind = urllib.parse.urlparse(handler.path).path.rsplit("/", 1)[1]
        if kind == "notifications":
            rows = [{"id": "a", "title": "Unread notice", "read_at": None, "body": "DETAIL"},
                    {"id": "b", "title": "Read notice", "read_at": "2026-10-01", "body": "DETAIL"}]
            return handler.reply(200, {"notifications": rows, "unread": 1})
        rows = [{"id": str(index), "title": state, "status": state, "body": "DETAIL",
                 "search_vector": "INTERNAL", "revision": 1, "project_slug": "fixture"}
                for index, state in enumerate(STATES.get(kind, []))]
        handler.reply(200, {kind: rows, "project": {"slug": "fixture", "rules": "DUPLICATE"}})

    def read(self, *args):
        with patch.object(Handler, "do_GET", self.get_response):
            result = self.cli("--project", "fixture", "--json", *args)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def query(self):
        return urllib.parse.parse_qs(urllib.parse.urlparse(Handler.calls[-1][1]).query)

    def test_active_defaults_are_lean_in_json_and_text_on_older_servers(self):
        expected = {"doc": ["current"], "plan": ["active"], "task": ["open", "in_progress"],
                    "feedback": ["open"], "watch": ["starting", "running"]}
        for command, statuses in expected.items():
            with self.subTest(command=command):
                kind = "watches" if command == "watch" else command + "s" if command != "feedback" else command
                data = self.read(command, "list")
                self.assertEqual([row["status"] for row in data[kind]], statuses)
                self.assertNotIn("DETAIL", json.dumps(data))
                self.assertNotIn("INTERNAL", json.dumps(data))
                self.assertEqual(self.query()["summary"], ["1"])
                with patch.object(Handler, "do_GET", self.get_response):
                    text = self.cli("--project", "fixture", command, "list")
                self.assertEqual(text.returncode, 0, text.stderr)
                self.assertNotIn("DETAIL", text.stdout)
                for status in set(STATES[kind]) - set(statuses):
                    self.assertNotIn(status, text.stdout)

    def test_explicit_all_and_status_do_not_imply_verbose(self):
        for command in ["doc", "plan", "task", "feedback", "watch"]:
            data = self.read(command, "list", "--all")
            kind = "watches" if command == "watch" else command + "s" if command != "feedback" else command
            self.assertEqual(len(data[kind]), len(STATES[kind]))
            self.assertNotIn("DETAIL", json.dumps(data))
            verbose = self.read(command, "list", "--all", "--verbose")
            self.assertEqual(verbose[kind][0]["body"], "DETAIL")
        outdated = self.read("doc", "list", "--status", "outdated")
        self.assertEqual([row["status"] for row in outdated["docs"]], ["outdated"])
        done = self.read("task", "list", "--status", "done")
        self.assertEqual([row["status"] for row in done["tasks"]], ["done"])

    def test_scope_flags_do_not_widen_lifecycle_visibility(self):
        for flags in [["--global"], ["--all-projects"], ["--local"]]:
            self.assertEqual([row["status"] for row in self.read("doc", "list", *flags)["docs"]], ["current"])
            self.assertEqual(self.query()["status"], ["current"])

    def test_search_is_lean_but_can_request_inactive_knowledge(self):
        data = self.read("search", "keyword")
        self.assertEqual([row["status"] for row in data["entries"]], ["active"])
        self.assertNotIn("DETAIL", json.dumps(data))
        self.assertNotIn("DUPLICATE", json.dumps(data))
        self.assertEqual(len(self.read("search", "keyword", "--all")["entries"]), 3)
        self.assertEqual(len(self.read("recent")["entries"]), 1)
        self.assertEqual(len(self.read("recent", "--all")["entries"]), 3)
        self.assertIn("DETAIL", json.dumps(self.read("search", "keyword", "--verbose")))

    def test_logs_keep_completed_work_and_literal_all_filters(self):
        data = self.read("log", "list")
        self.assertEqual([row["status"] for row in data["logs"]], STATES["logs"])
        self.assertNotIn("DETAIL", json.dumps(data))
        self.read("log", "list", "--grep", "all", "--agent", "all", "--status", "all")
        self.assertEqual(self.query()["grep"], ["all"])
        self.assertEqual(self.query()["agent"], ["all"])
        self.assertNotIn("status", self.query())

    def test_notifications_default_to_unread_with_explicit_read_or_all(self):
        self.assertEqual([row["id"] for row in self.read("notifications", "list")["notifications"]], ["a"])
        self.assertEqual(self.query()["state"], ["unread"])
        self.assertEqual([row["id"] for row in self.read("notifications", "list", "--read")["notifications"]], ["b"])
        self.assertEqual(len(self.read("notifications", "list", "--all")["notifications"]), 2)

    def test_conflicting_or_invalid_flags_fail_before_login(self):
        self.env.pop("CJ_TOKEN")
        self.config.unlink()
        for args in [("doc", "list", "--status", "done"), ("plan", "list", "--status", "current"),
                     ("task", "list", "--status", "blocked"), ("doc", "list", "--all", "--status", "current"),
                     ("notifications", "list", "--all", "--read")]:
            result = self.cli(*args)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn("not logged in", result.stderr)
