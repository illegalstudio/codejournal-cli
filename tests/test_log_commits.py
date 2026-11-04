import unittest
import test_session_feedback as sessions


class LogCommitsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sessions.SessionFeedbackTest.setUpClass()

    @classmethod
    def tearDownClass(cls):
        sessions.SessionFeedbackTest.tearDownClass()

    def setUp(self):
        self.fixture = sessions.SessionFeedbackTest(methodName="runTest")
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)

    def commit(self):
        case = self.fixture
        case.hook("SessionStart", "session-one")
        (case.repo / "mine.txt").write_text("mine\n")
        case.git("add", "mine.txt")
        output = case.git("commit", "-m", "My change")
        sha = case.git("rev-parse", "HEAD")
        return sha, output

    def test_explicit_short_commit_is_not_linked_again_automatically(self):
        case = self.fixture
        sha, output = self.commit()
        case.hook("PostToolUse", "session-one", "git commit -m 'My change'", output)
        case.cli("log", "add", "--title", "Done", "--body", "Done", "--ref", f"commit:{sha[:8]}")
        case.cli("log", "add", "--title", "Follow-up", "--body", "Follow-up")
        logs = [body for path, body in sessions.Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [])

    def test_explicit_log_before_post_tool_hook_marks_the_commit_logged(self):
        case = self.fixture
        sha, output = self.commit()
        case.cli("log", "add", "--title", "Done", "--body", "Done", "--ref", f"commit:{sha}")
        case.hook("PostToolUse", "session-one", "git commit -m 'My change'; cj log add", output)
        case.cli("log", "add", "--title", "Follow-up", "--body", "Follow-up")
        logs = [body for path, body in sessions.Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [])

    def test_uncaptured_head_is_explicitly_omitted_without_blocking_logs(self):
        import json
        import subprocess
        case = self.fixture
        sha, output = self.commit()
        status = subprocess.run([case.binary, "--json", "status"], cwd=case.repo,
            env=case.env, capture_output=True, text=True, check=True)
        self.assertIn("HEAD changed", json.loads(status.stdout)["notices"][0])
        for _ in range(2):
            failed = subprocess.run([case.binary, "log", "add", "--title", "Done", "--body", "Done"],
                cwd=case.repo, env=case.env, capture_output=True, text=True)
            self.assertEqual(failed.returncode, 0)
            self.assertIn("--ref commit:HEAD", failed.stdout)
            self.assertIn("No commits were inferred", failed.stdout)
            self.assertTrue(failed.stdout.splitlines()[-1].startswith("Logged "), failed.stdout)
        self.assertTrue(all(body["refs"] == [] for path, body in sessions.Handler.calls if path.endswith("/logs")))
        case.cli("log", "add", "--title", "Done", "--body", "Done", "--ref", "commit:HEAD")
        logs = [body for path, body in sessions.Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [{"kind": "commit", "value": sha}])
        case.hook("PostToolUse", "session-one", "git commit -m 'My change'; cj log add", output)
        case.cli("log", "add", "--title", "Next", "--body", "Next")
        logs = [body for path, body in sessions.Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [])

    def test_automatic_reference_overflow_keeps_the_remaining_commits_pending(self):
        import json
        case = self.fixture
        case.hook("SessionStart", "session-one")
        expected = []
        for index in range(35):
            (case.repo / "mine.txt").write_text(f"change {index}\n")
            case.git("add", "mine.txt")
            message = case.git("commit", "-m", f"change {index}")
            expected.append(case.git("rev-parse", "HEAD"))
            case.hook("PostToolUse", "session-one", "git commit -m change", message)
        first = json.loads(case.cli("log", "add", "--title", "First group", "--body", "Done", "--ref", "path:mine.txt", "--json"))
        self.assertEqual(first["commit_tracking"]["linked"], 29)
        self.assertEqual(first["commit_tracking"]["remaining"], 6)
        self.assertIn("remain pending", first["notices"][0])
        second = json.loads(case.cli("log", "add", "--title", "Remaining group", "--body", "Done", "--json"))
        self.assertEqual(second["commit_tracking"]["linked"], 6)
        self.assertEqual(second["commit_tracking"]["remaining"], 0)
        logs = [body for path, body in sessions.Handler.calls if path.endswith("/logs")]
        actual = [ref["value"] for log in logs for ref in log["refs"] if ref["kind"] == "commit"]
        self.assertEqual(actual, expected)
