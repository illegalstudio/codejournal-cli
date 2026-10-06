import json
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler


class BriefAuditTest(AuditCase):
    def response(self):
        huge = "irrelevant body " * 5000
        row = {"id": "a", "title": "Local work", "status": "active", "body": huge, "search_vector": huge}
        return {"project": {"id": "p", "slug": "fixture", "rules": "Do not duplicate rules"},
                "rules": "Complete rule\n" * 1000, "counts": {"active": 1},
                "plans": [row], "docs": [row], "tasks": [row], "recent": [row],
                "pinned": [row], "logs": [row], "global_docs": [row], "global": [row],
                "garden_hint": "Run garden", "other_sessions": [{"id": "s", "agent": "codex",
                    "branch": "feature", "host": "fixture", "checkout_path": "/work", "files": ["src/task.rs"]}]}

    def test_audit_is_lean_even_on_older_servers_and_has_an_isolated_cache(self):
        response = self.response()

        def respond(handler):
            body = json.loads(handler.rfile.read(int(handler.headers["Content-Length"])))
            Handler.calls.append(("POST", handler.path, body))
            handler.reply(200, response)

        with patch.object(Handler, "do_POST", respond):
            normal = self.cli("--project", "fixture", "--json", "brief")
            self.assertEqual(normal.returncode, 0, normal.stderr)
            audit = self.cli("--project", "fixture", "--json", "brief", "--audit")
            self.assertEqual(audit.returncode, 0, audit.stderr)
        data = json.loads(audit.stdout)
        self.assertEqual(data["rules"], response["rules"])
        self.assertEqual(data["plans"][0]["title"], "Local work")
        self.assertEqual(data["other_sessions"][0]["files"], ["src/task.rs"])
        for hidden in ["body", "search_vector", "global_docs", "garden_hint", "pinned", "recent", "logs"]:
            self.assertNotIn(f'"{hidden}"', audit.stdout)
        self.assertLess(len(audit.stdout), len(normal.stdout) // 10)
        request = Handler.calls[-1][2]
        self.assertTrue(request["audit"])
        for hidden in ["files", "provides_auto", "manifest_names"]:
            self.assertNotIn(hidden, request)
        cached = json.loads(self.cli("--project", "fixture", "--offline", "--json", "brief", "--audit").stdout)
        self.assertEqual(cached["rules"], response["rules"])
        self.assertEqual(cached["mode"], "offline")
        full = json.loads(self.cli("--project", "fixture", "--offline", "--json", "brief").stdout)
        self.assertIn("body", full["docs"][0])

    def test_zero_limits_are_explicit_and_invalid_limits_fail_before_login(self):
        result = self.cli("--project", "fixture", "brief", "--limit", "0", "--pinned-limit", "0", "--log-limit", "0")
        self.assertEqual(result.returncode, 0, result.stderr)
        request = Handler.calls[-1][2]
        self.assertEqual([request[name] for name in ["limit", "pinned_limit", "log_limit"]], [0, 0, 0])
        self.assertNotIn("Recent work", result.stdout)
        self.assertNotIn("Recent entries", result.stdout)
        self.env.pop("CJ_TOKEN")
        self.config.unlink()
        for flag in ["--limit", "--pinned-limit", "--log-limit"]:
            result = self.cli("brief", flag, "201")
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn("not logged in", result.stderr)
