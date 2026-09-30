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
