import json
import subprocess
import urllib.parse
from unittest.mock import patch

from feedback_audit_support import AuditCase, Handler, RESOURCE

FINDINGS = [{"id": f"f000000{i}-0000-4000-8000-000000000000", "kind": "stale_entry",
             "title": f"Knowledge {i}", "priority": 50, "targets": [{"kind": "entry", "id": RESOURCE}],
             "details": {"changes": 5}} for i in range(7)]
COUNTS = {"pending": 7, "reviewed": 0, "deferred": 0, "last_scan": None, "last_review": None}


def read(handler):
    Handler.calls.append(("GET", handler.path, None))
    parsed = urllib.parse.urlparse(handler.path)
    query = urllib.parse.parse_qs(parsed.query)
    if parsed.path.endswith("/garden/review"):
        after = query.get("after", [None])[0]
        start = next((i + 1 for i, row in enumerate(FINDINGS) if row["id"] == after), 0)
        limit = int(query.get("limit", [5])[0])
        rows = FINDINGS[start:start + limit]
        handler.reply(200, {"findings": rows, "counts": COUNTS,
                            "next": rows[-1]["id"] if start + limit < len(FINDINGS) else None})
    else:
        handler.reply(200, {"entries": [{"id": RESOURCE, "created_at": "2020-01-01T00:00:00Z",
                                         "refs": [{"kind": "path", "value": "README.md"}]}],
                            "docs": [], "next_page": None})


def write(handler):
    body = json.loads(handler.rfile.read(int(handler.headers["Content-Length"])))
    Handler.calls.append(("POST", handler.path, body))
    if handler.path.endswith("/maintenance"):
        handler.reply(200, {"applied": []})
    elif handler.path.endswith("/preview") or handler.path.endswith("/scan"):
        handler.reply(200, {"findings": FINDINGS if body.get("complete") else [], "reviewed": 0,
                            "deferred": 0, "progress": COUNTS, "automatic": {"secrets": 0, "topic_groups": 0}, "partial": False})
    else:
        handler.reply(200, {"finding": handler.path.rsplit("/", 1)[1], "outcome": body["outcome"],
                            "counts": {**COUNTS, "pending": 6}})


class GardenReviewTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.enterContext(patch.object(Handler, "do_GET", read))
        self.enterContext(patch.object(Handler, "do_POST", write))
        (self.repo / "README.md").write_text("Synthetic source\n")

    def run_json(self, *args):
        result = self.cli("--json", "--project", "fixture", *args)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_default_is_five_findings_and_all_and_continuation_are_explicit(self):
        result = self.run_json("garden")
        self.assertEqual(len(result["findings"]), 5)
        self.assertEqual(result["counts"]["pending"], 7)
        Handler.calls.clear()
        continued = self.run_json("garden", "--after", result["next"])
        self.assertEqual(len(continued["findings"]), 2)
        self.assertFalse(any(method == "POST" for method, _, _ in Handler.calls))
        self.assertEqual(len(self.run_json("garden", "--all")["findings"]), 7)

    def test_preview_and_cached_preview_do_not_write_and_remain_bounded(self):
        preview = self.run_json("garden", "--dry-run")
        self.assertEqual(len(preview["findings"]), 5)
        self.assertIsNone(preview["next"])
        self.assertEqual(len(self.run_json("garden", "--dry-run", "--all")["findings"]), 7)
        self.assertTrue(all(path.endswith("/preview") for method, path, _ in Handler.calls if method == "POST"))
        Handler.calls.clear()
        cached = self.run_json("--offline", "garden", "--dry-run")
        self.assertEqual(len(cached["findings"]), 5)
        self.assertTrue(cached["cached"])
        self.assertEqual(Handler.calls, [])

    def test_review_records_evidence_for_the_selected_version_and_rejects_invalid_options(self):
        result = self.run_json("garden", "review", FINDINGS[0]["id"], "--outcome", "verified", "--note", "Checked source")
        self.assertEqual(result["outcome"], "verified")
        body = Handler.calls[-1][2]
        self.assertEqual(body["note"], "Checked source")
        for args in [("--limit", "0"), ("review", FINDINGS[0]["id"], "--outcome", "wrong", "--note", "x"),
                     ("review", FINDINGS[0]["id"], "--outcome", "deferred", "--note", "x"),
                     ("review", FINDINGS[0]["id"], "--outcome", "verified", "--note", " ")]:
            Handler.calls.clear()
            self.assertNotEqual(self.cli("garden", *args).returncode, 0)
            self.assertEqual(Handler.calls, [])

    def git(self, *args):
        subprocess.run(["git", "-c", "commit.gpgsign=false", *args], cwd=self.repo, env=self.env, check=True, capture_output=True)

    def snapshot(self):
        Handler.calls.clear()
        self.run_json("garden", "--dry-run")
        return next(body["entries"][0]["code_hash"] for _, path, body in Handler.calls
                    if path.endswith("/preview") and body.get("entries"))

    def test_code_snapshot_tracks_content_including_uncommitted_changes_not_unrelated_commits(self):
        self.git("config", "user.email", "test@example.test")
        self.git("config", "user.name", "Test")
        self.git("add", "README.md")
        self.git("commit", "-m", "Initial")
        original = self.snapshot()
        (self.repo / "other.txt").write_text("Unrelated\n")
        self.git("add", "other.txt")
        self.git("commit", "-m", "Unrelated")
        self.assertEqual(self.snapshot(), original)
        (self.repo / "README.md").write_text("Changed synthetic source\n")
        changed = self.snapshot()
        self.assertNotEqual(changed, original)
        self.git("add", "README.md")
        self.git("commit", "-m", "Commit the reviewed content")
        self.assertEqual(self.snapshot(), changed)
