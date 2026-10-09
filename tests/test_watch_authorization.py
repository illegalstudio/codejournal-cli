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

    def test_multiline_whitespace_and_empty_arguments_start_and_finish_once(self):
        script = "\nimport sys\nprint(repr(sys.argv[1:]))\n"
        created = json.loads(self.cli("--json", "watch", "start", "--title", "Literal arguments",
                                      "--", "python3", "-c", script, "", "  spaced  "))
        result = self.wait(created["watch"]["id"], "finished")
        self.assertEqual(result["exit_code"], 0)
        self.assertIn("['', '  spaced  ']", result["tail"])
        stored = Handler.watches[created["watch"]["id"]]
        self.assertEqual(json.loads(stored["command"])[2:], [script, "", "  spaced  "])
        self.assertEqual(len(Handler.watches), 1)

    def test_mismatch_diagnostics_name_argument_index_without_values(self):
        private = "private-synthetic-argument"
        Handler.override = {"command": json.dumps(["true", private])}
        result = subprocess.run([self.binary, "--project", "p", "watch", "start", "--title", "Mismatch",
                                "--", "true", "authorized-value"], env=self.env, cwd=ROOT,
                                capture_output=True, text=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        ident = next(iter(Handler.watches))
        text = (pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches" / f"{ident}.runner.log").read_text()
        self.assertIn("command[1]", text)
        self.assertIn("Execution refused", text)
        self.assertNotIn(private, text)
        self.assertNotIn("authorized-value", text)
