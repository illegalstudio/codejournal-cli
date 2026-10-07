import json
import pathlib
import subprocess
from watch_support import WatchCase, Handler, ROOT


class WatchAuthorizationTest(WatchCase):
    def test_invalid_claim_response_preserves_unused_authorization_for_safe_recovery(self):
        Handler.heartbeat_override = {}
        marker = pathlib.Path(self.temp.name) / "executed"
        result = subprocess.run([self.binary, "--project", "p", "--json", "watch", "start", "--title", "Bad claim", "--", "touch", str(marker)],
                                env=self.env, cwd=ROOT, capture_output=True, text=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        state = pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches"
        self.assertEqual(len(list(state.glob("*.authorized.json"))), 1)
        self.assertFalse(marker.exists())
        Handler.heartbeat_override = None
        self.cli("sync")
        self.wait(next(iter(Handler.watches)), "finished")
        self.assertTrue(marker.exists())

    def test_invalid_fresh_status_retains_authorization_and_never_executes(self):
        Handler.override = {"status": None}
        marker = pathlib.Path(self.temp.name) / "executed"
        result = subprocess.run([self.binary, "--project", "p", "--json", "watch", "start", "--title", "Bad metadata", "--", "touch", str(marker)],
                                env=self.env, cwd=ROOT, capture_output=True, text=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        state = pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches"
        self.assertEqual(len(list(state.glob("*.authorized.json"))), 1)
        self.assertFalse(marker.exists())
        Handler.override = None
        self.cli("sync")
        self.wait(next(iter(Handler.watches)), "finished")
        self.assertTrue(marker.exists())

    def test_recovery_preserves_authorization_for_another_credential_scope(self):
        Handler.fail_creation = True
        marker = pathlib.Path(self.temp.name) / "executed"
        result = subprocess.run([self.binary, "--project", "p", "--json", "watch", "start", "--title", "Scope", "--", "touch", str(marker)],
                                env=self.env, cwd=ROOT, capture_output=True, text=True, timeout=15)
        data = json.loads(result.stdout)
        Handler.fail_creation = False
        self.env["CJ_TOKEN"] = "another-test-token"
        self.cli("sync")
        self.assertFalse(marker.exists())
        state = pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches"
        self.assertEqual(len(list(state.glob("*.authorized.json"))), 1)
        self.env["CJ_TOKEN"] = "test-token"
        self.cli("sync")
        self.wait(data["watch_id"], "finished")
        self.assertTrue(marker.exists())
