import json
import subprocess
from unittest.mock import patch
import test_garden_review as garden
from feedback_audit_support import AuditCase, Handler


class GardenEvidenceTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.enterContext(patch.object(Handler, "do_GET", garden.read))
        self.enterContext(patch.object(Handler, "do_POST", garden.write))

    def test_scan_carries_the_commits_and_paths_touching_the_cited_code(self):
        (self.repo / "README.md").write_text("Synthetic source\n")
        for args in [("add", "."), ("-c", "commit.gpgsign=false", "-c", "user.name=Synthetic", "-c", "user.email=test@example.invalid", "commit", "-m", "test: evidence")]:
            subprocess.run(["git", *args], cwd=self.repo, env=self.env, capture_output=True, check=True)
        sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=self.repo, capture_output=True, text=True, check=True).stdout.strip()
        result = self.cli("--project", "fixture", "garden", "--dry-run", "--json")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = [body["entries"][0] for method,path,body in Handler.calls if method == "POST" and path.endswith("/preview") and body.get("entries")]
        self.assertEqual(rows[0]["touching_commits"][0]["sha"], sha)
        self.assertIn("README.md", rows[0]["touching_commits"][0]["paths"])
        self.assertTrue(rows[0]["checked_branch"])

    def test_scan_omits_merge_commits_that_list_no_paths(self):
        def git(*args, check=True):
            return subprocess.run(["git", "-c", "commit.gpgsign=false", "-c", "user.name=Synthetic",
                                   "-c", "user.email=test@example.invalid", *args],
                                  cwd=self.repo, env=self.env, capture_output=True, text=True, check=check).stdout.strip()
        (self.repo / "README.md").write_text("Synthetic source\n")
        git("add", "."); git("commit", "-m", "test: base")
        git("checkout", "-q", "-b", "side"); (self.repo / "README.md").write_text("Side change\n")
        git("commit", "-am", "test: side"); git("checkout", "-q", "-")
        (self.repo / "README.md").write_text("Main change\n"); git("commit", "-am", "test: main")
        git("merge", "side", "-m", "test: merge", check=False)
        (self.repo / "README.md").write_text("Resolved\n"); git("add", "."); git("commit", "-m", "test: merge")
        merge = git("rev-parse", "HEAD")
        self.assertEqual(len(git("rev-list", "--parents", "-1", "HEAD").split()), 3, "fixture must end with a merge commit")
        result = self.cli("--project", "fixture", "garden", "--dry-run", "--json")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = [body["entries"][0] for method,path,body in Handler.calls if method == "POST" and path.endswith("/preview") and body.get("entries")]
        commits = rows[0]["touching_commits"]
        self.assertTrue(commits)
        self.assertTrue(all(commit["paths"] for commit in commits), commits)
        self.assertNotIn(merge, [commit["sha"] for commit in commits])

    def test_pending_output_shows_commit_paths_and_branch_scope(self):
        rows = json.loads(json.dumps(garden.FINDINGS))
        rows[0]["details"].update({"touching_commits":[{"sha":"a"*40,"paths":["src/cache.php"]}],
            "outside_branch":True,"branch_refs":["feature/other"],"checked_branch":"main"})
        self.enterContext(patch.object(garden, "FINDINGS", rows))
        result = self.cli("--project", "fixture", "garden", "--pending")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("a"*40, result.stdout)
        self.assertIn("src/cache.php", result.stdout)
        self.assertIn("feature/other", result.stdout)
