import json
import os
import pathlib
import subprocess
import tempfile
import unittest
import time
import hashlib

ROOT = pathlib.Path(__file__).resolve().parents[1]


class LocalFeedbackTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target/debug/cj")

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="cj-local-feedback-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.repo = self.base / "repo"
        self.repo.mkdir()
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(self.base / "config"),
            XDG_STATE_HOME=str(self.base / "state"), XDG_CACHE_HOME=str(self.base / "cache"),
            CJ_TOKEN="test-token", CJ_SESSION_ID="local-feedback", CJ_PROJECT="p",
            CODE_JOURNAL_HOOK_FLUSH="off")
        self.config = self.base / "config/codejournal/config.json"
        self.config.parent.mkdir(parents=True)
        self.config.write_text(json.dumps({"server": "http://127.0.0.1:1", "tenant": "demo"}))

    def cli(self, *args, payload=None, check=True):
        return subprocess.run([self.binary, "--json", *args], cwd=self.repo, env=self.env,
            input=json.dumps(payload) if payload else None, capture_output=True, text=True, check=check)

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.repo, capture_output=True, text=True, check=True).stdout.strip()

    def test_outbox_is_inspectable_and_droppable_without_authentication(self):
        secret = "ghp_" + "Z9y8" * 9
        queued = json.loads(self.cli("--offline", "add", "--kind", "gotcha", "--title", "Note", "--body", secret).stdout)
        self.config.unlink()
        self.env.pop("CJ_TOKEN")
        result = json.loads(self.cli("outbox", "list", "--body").stdout)
        self.assertEqual(result["requests"][0]["id"], queued["id"])
        self.assertNotIn(secret, json.dumps(result))
        self.assertEqual(result["requests"][0]["body"]["body"], "[redacted github token]")
        self.assertEqual(json.loads(self.cli("outbox", "list", "--server", "https://other.test").stdout)["requests"], [])
        self.cli("outbox", "drop", queued["id"][:8])
        self.assertEqual(json.loads(self.cli("outbox", "list").stdout)["requests"], [])

    def test_host_refs_are_explicit_urls_and_repository_paths_stay_restricted(self):
        self.git("init", "-q")
        self.cli("--offline", "add", "--kind", "gotcha", "--title", "Setup", "--body", "Synthetic",
            "--ref", "file:~/.config/example.conf", "--ref", "host:example.test:/etc/example.conf")
        request = json.loads(next((self.base / "state").rglob("requests/*.json")).read_text())
        refs = request["body"]["refs"]
        self.assertEqual([item["kind"] for item in refs], ["url", "url"])
        self.assertIn("/.config/example.conf", refs[0]["value"])
        self.assertEqual(refs[1]["value"], "file://example.test/etc/example.conf")
        failed = self.cli("--offline", "add", "--kind", "gotcha", "--title", "Bad", "--body", "Synthetic",
            "--ref", "path:/tmp/outside.conf", check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("file:/absolute/path", failed.stderr)

    def test_combined_commit_log_has_no_stale_links_and_queued_commit_is_not_relinked(self):
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "test@example.test")
        self.git("config", "user.name", "Test")
        self.git("config", "commit.gpgsign", "false")
        (self.repo / "file").write_text("Initial")
        self.git("add", "file")
        self.git("commit", "-m", "Initial")
        payload = {"session_id": "local-feedback", "cwd": str(self.repo)}
        self.cli("hook", "SessionStart", payload=payload)
        self.wait_for_activity(1)
        (self.repo / "file").write_text("Updated")
        self.git("add", "file")
        output = self.git("commit", "-m", "Updated")
        sha = self.git("rev-parse", "HEAD")
        failed = self.cli("--offline", "log", "add", "--title", "Done", "--body", "Done", check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("--ref commit:HEAD", failed.stderr)
        self.cli("--offline", "log", "add", "--title", "Done", "--body", "Done", "--ref", "commit:HEAD")
        self.cli("hook", "PostToolUse", payload=dict(payload, tool_input={"command": "git commit -m Updated; cj log add"},
            tool_response={"stdout": output, "exit_code": 0}))
        self.wait_for_activity(2)
        self.cli("--offline", "log", "add", "--title", "Next", "--body", "Next")
        requests = [json.loads(path.read_text()) for path in sorted((self.base / "state").rglob("requests/*.json"))]
        requests = [request for request in requests if request["path"].endswith("/logs")]
        self.assertEqual(requests[0]["body"]["refs"], [{"kind": "commit", "value": sha}])
        self.assertEqual(requests[1]["body"]["refs"], [])

    def test_session_start_warns_before_first_write_when_token_is_unavailable(self):
        self.env.pop("CJ_TOKEN")
        result = self.cli("hook", "SessionStart", payload={"session_id": "local-feedback", "cwd": str(self.repo)})
        self.assertIn("authentication is unavailable", result.stdout)
        self.assertIn("cannot save writes", result.stdout)

    def wait_for_activity(self, expected):
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            requests = [json.loads(path.read_text()) for path in (self.base / "state").rglob("requests/*.json")]
            if sum(request["path"].endswith("/checkout-activity") for request in requests) >= expected:
                return
            time.sleep(0.01)
        self.fail("Detached test activity publisher did not finish")

    def test_authentication_notice_and_cached_brief_form_one_hook_response(self):
        cache = self.base / "state/codejournal/briefs" / (hashlib.sha256(str(self.repo).encode()).hexdigest() + ".txt")
        cache.parent.mkdir(parents=True)
        cache.write_text("Cached synthetic project brief")
        self.env.pop("CJ_TOKEN")
        result = self.cli("hook", "SessionStart", payload={"session_id": "local-feedback", "cwd": str(self.repo)})
        context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("authentication is unavailable", context)
        self.assertIn("Cached synthetic project brief", context)
        self.assertEqual(cache.read_text(), "Cached synthetic project brief")
