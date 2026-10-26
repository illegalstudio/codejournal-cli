import json
import subprocess
from unittest.mock import patch

import test_garden_review as modern
from feedback_audit_support import AuditCase, Handler, RESOURCE


class GardenCompatibilityTest(AuditCase):
    def test_legacy_report_is_bounded_and_cannot_claim_persistent_reviews(self):
        def read(handler):
            Handler.calls.append(("GET", handler.path, None))
            if handler.path.endswith('/garden/report'):
                handler.reply(200, {"reported_wrong": [{"id": RESOURCE, "title": f"Fact {i}", "wrong": 1}
                                                      for i in range(12)], "topic_counts": {"unnecessary": 100}})
            else:
                handler.reply(200, {"entries": [], "docs": []})
        with patch.object(Handler, "do_GET", read):
            result = self.cli("--json", "--project", "fixture", "garden", "--dry-run")
            self.assertEqual(result.returncode, 0, result.stderr)
            data = json.loads(result.stdout)
            self.assertEqual(len(data["reported_wrong"]), 5)
            self.assertEqual(data["counts"], {"pending": 12, "shown": 5})
            self.assertFalse(data["review_supported"])
            self.assertNotIn("topic_counts", data)
            self.assertIn("does not support persistent", result.stderr)
            all_result = self.cli("--json", "--project", "fixture", "garden", "--dry-run", "--all")
            self.assertEqual(len(json.loads(all_result.stdout)["reported_wrong"]), 12)
            continuation = self.cli("--project", "fixture", "garden", "--after", modern.FINDINGS[0]["id"])
            self.assertNotEqual(continuation.returncode, 0)
            self.assertIn("does not support persistent", continuation.stderr)

    def test_directory_snapshot_is_stable_when_reviewed_files_are_committed(self):
        def read(handler):
            if '/garden?' in handler.path:
                handler.reply(200, {"entries": [{"id": RESOURCE, "refs": [{"kind": "path", "value": "src"}]}], "docs": []})
            else:
                modern.read(handler)
        def git(*args):
            subprocess.run(["git", "-c", "commit.gpgsign=false", *args], cwd=self.repo, env=self.env,
                           check=True, capture_output=True)
        def snapshot():
            Handler.calls.clear()
            result = self.cli("--project", "fixture", "garden", "--dry-run")
            self.assertEqual(result.returncode, 0, result.stderr)
            return next(body["entries"][0]["code_hash"] for _, path, body in Handler.calls
                        if path.endswith("/preview") and body.get("entries"))
        git("config", "user.email", "test@example.test")
        git("config", "user.name", "Test")
        (self.repo / "src").mkdir()
        (self.repo / "src/file.rs").write_text("Initial\n")
        git("add", "src")
        git("commit", "-m", "Initial")
        with patch.object(Handler, "do_GET", read), patch.object(Handler, "do_POST", modern.write):
            original = snapshot()
            (self.repo / "src/untracked.rs").write_text("Reviewed new source\n")
            changed = snapshot()
            self.assertNotEqual(changed, original)
            git("add", "src")
            git("commit", "-m", "Commit reviewed source")
            self.assertEqual(snapshot(), changed)
            (self.repo / "src/file.rs").unlink()
            deleted = snapshot()
            self.assertNotEqual(deleted, changed)
            git("add", "src")
            git("commit", "-m", "Commit reviewed deletion")
            self.assertEqual(snapshot(), deleted)
