import subprocess
from auto_sync_support import AutoSyncCase, Handler


class AutoSyncActivityTest(AutoSyncCase):
    def test_checkout_transitions_are_delivered_in_order_and_identical_states_are_skipped(self):
        repo = self.base / "repo"
        repo.mkdir()
        for args in [("init", "-q", "-b", "main"), ("config", "user.name", "Test"),
                     ("config", "user.email", "test@example.test"), ("config", "commit.gpgsign", "false")]:
            subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True)
        file = repo / "file"
        file.write_text("Initial")
        subprocess.run(["git", "add", "file"], cwd=repo, check=True, capture_output=True)
        subprocess.run(["git", "commit", "-qm", "Initial"], cwd=repo, check=True, capture_output=True)
        def publish():
            return self.cli("--cwd", str(repo), "--offline", "activity", "publish")
        publish()
        publish()
        self.assertEqual(len(self.queue()), 1)
        file.write_text("Changed")
        publish()
        file.write_text("Initial")
        publish()
        self.assertEqual(len(self.queue()), 3)
        self.cli("projects")
        self.wait_for(lambda: not self.queue())
        self.assertEqual([body["dirty"] for _, body in Handler.applied], [0, 1, 0])
        self.cli("--cwd", str(repo), "activity", "publish")
        self.assertEqual(len(Handler.calls), 3)
