import json
import time
import unittest
import test_session_feedback as sessions


class SessionOwnershipTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sessions.SessionFeedbackTest.setUpClass()

    @classmethod
    def tearDownClass(cls):
        sessions.SessionFeedbackTest.tearDownClass()

    def setUp(self):
        self.case = sessions.SessionFeedbackTest(methodName="runTest")
        self.case.setUp()
        self.addCleanup(self.case.doCleanups)

    def test_other_checkout_activity_does_not_claim_commits_or_block_a_log(self):
        case = self.case
        case.hook("SessionStart", "session-one")
        (case.repo / "external").write_text("Other actor")
        case.git("add", "external")
        case.git("commit", "-m", "Other actor")
        self.assertEqual(case.cli("hook", "Stop", payload={"session_id": "session-one", "cwd": str(case.repo)}), "")
        result = json.loads(case.cli("--json", "log", "add", "--title", "Read only", "--body", "Inspected"))
        self.assertEqual(result["log"]["refs"], [])
        self.assertIn("No commits were inferred", result["notices"][0])

    def test_dry_run_commit_does_not_capture_the_current_head(self):
        case = self.case
        case.hook("SessionStart", "session-one")
        case.hook("PostToolUse", "session-one", "git commit --dry-run", "On branch main")
        case.cli("log", "add", "--title", "Inspected", "--body", "No commit")
        body = next(body for path, body in sessions.Handler.calls if path.endswith("/logs"))
        self.assertEqual(body["refs"], [])

    def test_only_body_amendments_of_this_sessions_log_satisfy_the_stop_hook(self):
        case = self.case
        case.hook("SessionStart", "session-one")
        log = json.loads(case.cli("--json", "log", "add", "--title", "Session work", "--body", "Initial"))["log"]
        time.sleep(1.05)
        case.hook("UserPromptSubmit", "session-one")
        edit = {"session_id": "session-one", "cwd": str(case.repo), "tool_name": "Edit",
                "tool_input": {"file_path": "README.md"}}
        case.cli("hook", "PostToolUse", payload=edit)
        case.cli("log", "update", "dddddddd-dddd-4ddd-8ddd-dddddddddddd", "--base", "1", "--body", "Other session")
        stop = {"session_id": "session-one", "cwd": str(case.repo)}
        self.assertEqual(json.loads(case.cli("hook", "Stop", payload=stop))["decision"], "block")
        case.cli("log", "update", log["id"], "--base", "1", "--body", "Completed")
        case.hook("UserPromptSubmit", "session-one")
        case.cli("hook", "PostToolUse", payload=edit)
        case.cli("log", "update", log["id"], "--base", "2", "--body", "Continued")
        self.assertEqual(case.cli("hook", "Stop", payload=stop), "")
