"""Tasks, feedback, work logs, and notifications through both real backends."""
from support import ParityCase


class WorkflowParityTest(ParityCase):
    def test_task_lifecycle_links_notes_and_priority(self):
        tasks = self.paired("task", "add", "--title", "Repair cache", "--body", "Synthetic details", "--priority", "high")
        for engine, result in tasks.items():
            identity = result["task"]["id"]
            self.invoke(engine, "task", "start", identity, "--note", "Investigating")
            self.assertEqual(self.invoke(engine, "task", "show", identity)["task"]["status"], "in_progress")
            self.invoke(engine, "task", "note", identity, input="Found the cause\n")
            self.invoke(engine, "task", "link", identity, "illegalstudio/codejournal#12")
            task = self.invoke(engine, "task", "show", identity)
            self.assertIn({"kind": "url", "value": "https://github.com/illegalstudio/codejournal/issues/12"}, task["task"]["refs"])
            self.assertIn("Found the cause", [event["note"] for event in task["events"]])
            self.invoke(engine, "task", "unlink", identity, "illegalstudio/codejournal#12")
            self.invoke(engine, "task", "edit", identity, "--priority", "low", "--not-before", "2099-02-03")
            task = self.invoke(engine, "task", "show", identity)["task"]
            self.assertEqual((task["priority"], task["not_before"]), ("low", "2099-02-03"))
            self.invoke(engine, "task", "done", identity, "--note", "Fixed")
        self.titles(self.paired("task", "list"), "tasks", [])
        self.titles(self.paired("task", "list", "--status", "closed"), "tasks", ["Repair cache"])
        for engine, result in tasks.items():
            self.invoke(engine, "task", "reopen", result["task"]["id"])
            self.invoke(engine, "task", "dismiss", result["task"]["id"], "--note", "No longer needed")

    def test_feedback_lifecycle_and_duplicates(self):
        feedback = self.paired("feedback", "add", "--category", "workflow", "--title", "Explain workflow", "--body", "Synthetic suggestion")
        for engine, result in feedback.items():
            identity = result["feedback"]["id"]
            self.assertNotEqual(self.invoke(engine, "feedback", "add", "--category", "workflow", "--title", "Explain workflow",
                                           "--body", "Duplicate", check=False).returncode, 0)
            self.invoke(engine, "feedback", "close", identity, "--note", "Implemented")
        self.titles(self.paired("feedback", "list"), "feedback", [])
        self.titles(self.paired("feedback", "list", "--status", "done", "--category", "workflow"), "feedback", ["Explain workflow"])
        for engine, result in feedback.items():
            self.invoke(engine, "feedback", "reopen", result["feedback"]["id"])
            self.invoke(engine, "feedback", "dismiss", result["feedback"]["id"], "--note", "Duplicate request")

    def test_log_filters_and_notifications(self):
        self.paired("log", "add", "--title", "Cache shipped", "--body", "Verified synthetic cache",
                    "--status", "done", "--agent", "codex", "--no-auto-commits", "--ref", "path:README.md")
        self.titles(self.paired("log", "list", "--since", "yesterday", "--grep", "cache", "--agent", "codex", "--status", "done"),
                    "logs", ["Cache shipped"])
        notifications = self.paired("notify", "--kind", "needs_input", "--title", "Choose cache", "--body", "Synthetic decision")
        for engine, result in notifications.items():
            listed = self.invoke(engine, "notifications", "list", "--unread", "--project-only")
            self.assertEqual([item["title"] for item in listed["notifications"]], ["Choose cache"])
            identity = listed["notifications"][0]["id"]
            self.invoke(engine, "notifications", "read", identity)
            self.assertEqual(self.invoke(engine, "notifications", "list", "--unread")["notifications"], [])
            self.invoke(engine, "notifications", "unread", identity)
            self.assertEqual(len(self.invoke(engine, "notifications", "list", "--unread")["notifications"]), 1)
