import hashlib
import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler


class DiscoveryBriefTest(AuditCase):
    @staticmethod
    def respond(handler):
        request = json.loads(handler.rfile.read(int(handler.headers["Content-Length"])))
        Handler.calls.append(("POST", handler.path, request))
        handler.reply(200, {"project": {"slug": "fixture", "rules": "DUPLICATE"}, "rules": "Complete known rules",
            "plans": [{"id": "a", "title": "Active plan", "status": "active", "body": "DETAIL"},
                      {"id": "d", "title": "Draft plan", "status": "draft"}],
            "docs": [{"id": "c", "title": "Current doc", "status": "current", "body": "DETAIL"},
                     {"id": "o", "title": "Outdated doc", "status": "outdated"}],
            "tasks": [{"title": "Ongoing task", "status": "in_progress"}, {"title": "Closed task", "status": "done"}],
            "recent": [{"title": "Current entry", "status": "active", "body": "DETAIL"},
                       {"title": "Obsolete entry", "status": "obsolete"}],
            "logs": [{"title": "Completed useful work", "status": "done", "body": "DETAIL"}],
            "forwarded_results": [{"title": "Closed forwarding", "status": "done"}],
            "focus": {"docs": [{"title": "Stale focused doc", "status": "outdated"}]}})

    def read(self, *flags):
        result = self.cli("--project", "fixture", "--json", "brief", *flags)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result, json.loads(result.stdout)

    def test_default_and_explicit_cache_modes_are_isolated(self):
        with patch.object(Handler, "do_POST", self.respond):
            default, data = self.read()
            self.assertEqual(data["rules"], "Complete known rules")
            self.assertEqual(data["logs"][0]["status"], "done")
            for forbidden in ["DETAIL", "DUPLICATE", "Draft plan", "Outdated doc", "Closed task", "Obsolete entry", "Stale focused doc"]:
                self.assertNotIn(forbidden, default.stdout)
            self.assertNotIn("entries", data)
            self.assertEqual(Handler.calls[-1][2]["visibility"], "active")
            self.assertTrue(Handler.calls[-1][2]["summary"])
            full, _ = self.read("--all", "--verbose")
            self.assertIn("Outdated doc", full.stdout)
            self.assertIn("DETAIL", full.stdout)
        default_cached, data = self.read("--offline")
        self.assertEqual(data["mode"], "offline")
        self.assertNotIn("Outdated doc", default_cached.stdout)
        self.assertNotIn("DETAIL", default_cached.stdout)
        full_cached, _ = self.read("--offline", "--all", "--verbose")
        self.assertIn("Outdated doc", full_cached.stdout)
        self.assertIn("DETAIL", full_cached.stdout)

    def test_broad_cache_cannot_supply_default_offline_context(self):
        with patch.object(Handler, "do_POST", self.respond):
            self.read("--all", "--verbose")
        result = self.cli("--project", "fixture", "--json", "--offline", "brief")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Outdated doc", result.stdout)

    def test_hook_requests_active_summaries_and_never_reuses_legacy_text(self):
        cache = self.base / "state/codejournal/briefs" / (hashlib.sha256(str(self.repo).encode()).hexdigest() + ".txt")
        cache.parent.mkdir(parents=True)
        cache.write_text("Legacy inactive Outdated doc")
        with patch.object(Handler, "do_POST", self.respond):
            result = self.cli("hook", "SessionStart", stdin=json.dumps({"session_id": "discovery", "cwd": str(self.repo)}))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Complete known rules", result.stdout)
        self.assertIn("Completed useful work", result.stdout)
        self.assertNotIn("Draft plan", result.stdout)
        self.assertNotIn("Outdated doc", result.stdout)
        self.assertEqual(Handler.calls[-1][2]["visibility"], "active")
        self.assertTrue(Handler.calls[-1][2]["summary"])
        self.assertEqual(cache.read_text(), "Legacy inactive Outdated doc")
