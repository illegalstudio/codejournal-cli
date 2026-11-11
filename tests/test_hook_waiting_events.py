import time
import unittest
import test_session_feedback as sessions


class HookWaitingEventsTest(unittest.TestCase):
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

    def test_waiting_prompts_record_session_activity_without_notifications(self):
        case = self.case
        case.env["CODE_JOURNAL_HOOK_SYNC"] = "1"
        case.hook("SessionStart", "session-wait")
        base = {"session_id": "session-wait", "cwd": str(case.repo)}
        prompts = [("idle_prompt", "Claude is waiting for your input"),
                   ("permission_prompt", "Claude needs your permission to use Bash"),
                   ("elicitation_dialog", "Claude has a question"),
                   ("agent_needs_input", "OpenCode needs your input")]
        for kind, message in prompts:
            case.cli("hook", "Notification", payload={**base, "notification_type": kind, "message": message},
                     session="session-wait")
        tool = {**base, "tool_name": "Bash", "tool_use_id": "tool-one"}
        case.cli("hook", "PermissionRequest", payload=tool, session="session-wait")
        case.cli("hook", "PostToolUse", payload={**tool, "tool_input": {"command": "true"},
                 "tool_response": {"exit_code": 0}}, session="session-wait")
        time.sleep(1)
        self.assertEqual([path for path, _ in sessions.Handler.calls if path.endswith("/notifications")], [])
        events = [event for path, body in sessions.Handler.calls if path.endswith("/client-events")
                  for event in body["events"]]
        self.assertEqual([event["reason"] for event in events if event["type"] == "waiting"],
                         [message for _, message in prompts] + ["approval needed for Bash"])
        self.assertEqual(events[-1]["type"], "tool")
        self.assertTrue(all("resolved_notification_id" not in event for event in events))
