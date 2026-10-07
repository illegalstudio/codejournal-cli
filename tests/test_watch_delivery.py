import json
import pathlib
import time
from watch_support import WatchCase, Handler


class WatchDeliveryTest(WatchCase):
    def test_queued_outcome_recovers_without_repeating_the_command_or_notification(self):
        Handler.fail_result = True
        marker = pathlib.Path(self.temp.name) / "executions"
        command = ["sh", "-c", f"printf 'run\\n' >> '{marker}'"]
        data = json.loads(self.cli("--json", "watch", "start", "--title", "Delayed outcome", "--", *command))
        ident = data["watch"]["id"]
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline:
            status = json.loads(self.cli("status", "--json"))
            if status["pending_outbox"] == 1:
                break
            time.sleep(0.05)
        self.assertEqual(status["pending_outbox"], 1)
        self.assertEqual(marker.read_text(), "run\n")
        self.assertEqual(Handler.watches[ident]["status"], "running")
        Handler.fail_result = False
        Handler.fail_listing = True
        synced = json.loads(self.cli("--json", "sync"))
        self.assertTrue(synced["synced"])
        self.assertEqual(synced["flushed"], 1)
        self.assertEqual(synced["pending_outbox"], 0)
        self.assertFalse(synced["watches_reconciled"])
        self.assertIn("Watch reconciliation deferred", synced["notices"][0])
        self.wait(ident, "finished")
        Handler.fail_listing = False
        self.cli("sync")
        self.assertEqual(marker.read_text(), "run\n")
        self.assertEqual(len(Handler.notifications), 1)
        attempts = [key for method, path, key in Handler.paths if method == "PATCH" and path.endswith(ident)]
        self.assertEqual(len(attempts), 2)
        self.assertEqual(attempts[0], attempts[1])
