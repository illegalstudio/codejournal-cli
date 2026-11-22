import unittest
import test_session_feedback as sessions


class SessionGitHeadTest(unittest.TestCase):
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

    def hook(self, event, session, checkout):
        self.case.cli("hook", event, session=session, cwd=checkout, payload={
            "session_id": session, "cwd": str(checkout), "tool_name": "Bash"
        })

    def test_hooks_report_checkout_branches_and_changes_without_extra_heartbeats(self):
        case = self.case
        worktree = case.base / "worktree"
        case.git("worktree", "add", "-b", "feat/worktree", str(worktree))
        self.hook("SessionStart", "main-session", case.repo)
        self.hook("SessionStart", "worktree-session", worktree)
        self.hook("PostToolUse", "worktree-session", worktree)

        case.git("switch", "-c", "feat/changed", cwd=worktree)
        self.hook("PostToolUse", "worktree-session", worktree)
        self.hook("PostToolUse", "worktree-session", worktree)
        case.git("switch", "--detach", cwd=worktree)
        self.hook("PostToolUse", "worktree-session", worktree)
        case.git("switch", "feat/worktree", cwd=worktree)
        self.hook("UserPromptSubmit", "worktree-session", worktree)
        self.hook("SessionStart", "outside-git", case.base)

        events = [event for path, body in sessions.Handler.calls if path.endswith("/client-events")
                  for event in body["events"]]
        self.assertEqual([(event["session"], event["type"], event.get("git_head")) for event in events], [
            ("main-session", "start", {"branch": "main"}),
            ("worktree-session", "start", {"branch": "feat/worktree"}),
            ("worktree-session", "tool", {"branch": "feat/changed"}),
            ("worktree-session", "tool", {"branch": None}),
            ("worktree-session", "prompt", {"branch": "feat/worktree"}),
            ("outside-git", "start", None),
        ])
