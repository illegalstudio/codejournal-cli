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

    def test_same_shell_requires_explicit_head_before_hook(self):
        import subprocess
        case = self.fixture
        sha, output = self.commit()
        for _ in range(2):
            failed = subprocess.run([case.binary, "log", "add", "--title", "Done", "--body", "Done"],
                cwd=case.repo, env=case.env, capture_output=True, text=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("--ref commit:HEAD", failed.stderr)
            self.assertIn("Repeating the command alone cannot recover missing hook data", failed.stderr)
        self.assertFalse(any(path.endswith("/logs") for path, _ in sessions.Handler.calls))
        case.cli("log", "add", "--title", "Done", "--body", "Done", "--ref", "commit:HEAD")
        logs = [body for path, body in sessions.Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [{"kind": "commit", "value": sha}])
        case.hook("PostToolUse", "session-one", "git commit -m 'My change'; cj log add", output)
        case.cli("log", "add", "--title", "Next", "--body", "Next")
        logs = [body for path, body in sessions.Handler.calls if path.endswith("/logs")]
        self.assertEqual(logs[-1]["refs"], [])
