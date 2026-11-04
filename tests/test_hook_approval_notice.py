import time
import unittest
import test_session_feedback as sessions


class HookApprovalNoticeTest(unittest.TestCase):
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

    def test_only_the_exact_completed_tool_resolves_its_automatic_notice(self):
        case = self.case
        case.hook("SessionStart", "session-one")
        payload = {"session_id":"session-one", "cwd":str(case.repo), "tool_name":"Bash", "tool_use_id":"tool-one"}
        case.cli("hook", "PermissionRequest", payload=payload)
        deadline = time.monotonic()+5
        notices = []
        while time.monotonic() < deadline:
            notices = [body for path,body in sessions.Handler.calls if path.endswith("/notifications")]
            if notices:
                break
            time.sleep(.04)
        self.assertEqual(len(notices), 1)
        self.assertEqual(notices[0]["source"], "hook_approval")
        notice_id = notices[0]["id"]
        case.cli("hook", "PostToolUse", payload={**payload,"tool_use_id":"different-tool","tool_input":{"command":"true"},"tool_response":{"exit_code":0}})
        unrelated = [event for path,body in sessions.Handler.calls if path.endswith("/client-events") for event in body["events"]]
        self.assertTrue(all(event.get("resolved_notification_id") != notice_id for event in unrelated))
        case.cli("hook", "PostToolUse", payload={**payload,"tool_input":{"command":"true"},"tool_response":{"exit_code":0}})
        events = [event for path,body in sessions.Handler.calls if path.endswith("/client-events") for event in body["events"]]
        self.assertTrue(any(event.get("resolved_notification_id") == notice_id for event in events))
