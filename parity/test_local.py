"""Local Git attribution and detached watch outcomes with synthetic repositories."""
import time
from support import ParityCase


class LocalParityTest(ParityCase):
    def test_git_refs_branch_and_commit_resolution(self):
        self.git("checkout", "-b", "feature/parity")
        sha = self.git("rev-parse", "HEAD")
        created = self.paired("add", "--kind", "gotcha", "--title", "Branch knowledge", "--body", "Synthetic branch",
                              "--ref", f"commit:{sha}", "--ref", "illegalstudio/codejournal#12", project=None)
        for engine, result in created.items():
            self.assertIn("entry", result, f"{engine}: {result}")
            entry = self.invoke(engine, "show", result["entry"]["id"], "--no-track")["entry"]
            self.assertEqual(sorted((ref["kind"], ref["value"]) for ref in entry["refs"]),
                             [("branch", "feature/parity"), ("commit", sha),
                              ("url", "https://github.com/illegalstudio/codejournal/issues/12")])
            logged = self.invoke(engine, "log", "add", "--title", "Branch work", "--body", "Synthetic log",
                                 "--ref", f"commit:{sha}", "--no-auto-commits")
            self.assertIn({"kind": "commit", "value": sha}, logged["log"]["refs"])
        self.git("checkout", "main")
        self.titles(self.paired("search", "branch"), "entries", ["Branch knowledge"])

    def test_watch_success_failure_and_failure_only_policy(self):
        for engine in self.envs:
            self.invoke(engine, "notifications", "config", "--desktop", "off")
            for label, code, policy in [("Success", 0, "end"), ("Failure", 3, "failure"), ("Quiet success", 0, "failure")]:
                self.invoke(engine, "watch", "start", "--title", label, "--notify-on", policy,
                            "--", "sh", "-c", f"printf synthetic; exit {code}")
                deadline = time.monotonic() + 8
                while time.monotonic() < deadline:
                    watches = self.invoke(engine, "watch", "list", "--all")["watches"]
                    item = next(item for item in watches if item["title"] == label)
                    if item["status"] == "finished":
                        break
                    time.sleep(0.05)
                self.assertEqual(item["status"], "finished", engine)
                self.assertEqual(item["exit_code"], code)
                if label != "Quiet success":
                    while time.monotonic() < deadline:
                        notes = self.invoke(engine, "notifications", "list")["notifications"]
                        if any(note["title"].startswith(label + ":") for note in notes):
                            break
                        time.sleep(0.05)
                    self.assertTrue(any(note["title"].startswith(label + ":") for note in notes))
            notes = self.invoke(engine, "notifications", "list")["notifications"]
            self.assertFalse(any(note["title"].startswith("Quiet success:") for note in notes))
