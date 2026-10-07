import json
import pathlib
import socket
import subprocess
import uuid
from watch_support import WatchCase, Handler, ROOT


class WatchRecoveryTest(WatchCase):
    def result(self, *args):
        return subprocess.run([self.binary, "--project", "p", *args], env=self.env,
                              cwd=ROOT, capture_output=True, text=True, timeout=15)

    def marker_command(self):
        marker = pathlib.Path(self.temp.name) / "executions"
        return marker, ["sh", "-c", f"printf 'run\\n' >> '{marker}'"]

    def authorizations(self):
        return list((pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches").glob("*.authorized.json"))

    def test_queued_start_exits_unsuccessfully_and_sync_executes_once_with_original_request(self):
        Handler.fail_creation = True
        marker, command = self.marker_command()
        result = self.result("--json", "watch", "start", "--title", "Queued", "--", *command)
        self.assertNotEqual(result.returncode, 0)
        receipt = json.loads(result.stdout)
        self.assertTrue(receipt["queued"])
        self.assertFalse(receipt["started"])
        self.assertFalse(marker.exists())
        self.assertEqual(len(self.authorizations()), 1)
        request_id = receipt["request_id"]
        Handler.fail_creation = False
        self.cli("sync")
        self.wait(receipt["watch_id"], "finished")
        self.cli("sync")
        self.assertEqual(marker.read_text(), "run\n")
        self.assertFalse(self.authorizations())
        attempts = [key for method, path, key in Handler.paths if method == "POST" and path.endswith("/watches")]
        self.assertEqual(attempts, [request_id, request_id])
        self.assertEqual(len(Handler.watches), 1)

    def test_uncertain_creation_retries_the_receipt_without_creating_or_executing_twice(self):
        Handler.accepted_nonjson = True
        marker, command = self.marker_command()
        result = self.result("--json", "watch", "start", "--title", "Uncertain", "--", *command)
        self.assertNotEqual(result.returncode, 0)
        receipt = json.loads(result.stdout)
        self.assertEqual(len(Handler.watches), 1)
        self.assertFalse(marker.exists())
        self.cli("sync")
        self.wait(receipt["watch_id"], "finished")
        self.cli("sync")
        self.assertEqual(len(Handler.watches), 1)
        self.assertEqual(marker.read_text(), "run\n")

    def test_failed_initial_read_keeps_unused_authorization_and_safe_http_diagnostics(self):
        Handler.fail_detail = True
        marker, command = self.marker_command()
        result = self.result("--json", "watch", "start", "--title", "Read outage", "--", *command)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(marker.exists())
        self.assertEqual(len(self.authorizations()), 1)
        watch = next(iter(Handler.watches.values()))
        self.assertEqual(watch["status"], "starting")
        log = self.authorizations()[0].parent / f"{watch['id']}.runner.log"
        text = log.read_text()
        self.assertIn("HTTP 503", text)
        self.assertIn("Content-Type text/html", text)
        self.assertNotIn("Private synthetic", text)
        Handler.fail_detail = False
        self.cli("sync")
        self.wait(watch["id"], "finished")
        self.cli("sync")
        self.assertEqual(marker.read_text(), "run\n")

    def test_cancel_missing_local_record_does_not_load_history(self):
        ident = str(uuid.uuid4())
        marker, command = self.marker_command()
        Handler.watches[ident] = {"id": ident, "status": "running", "title": "Orphan",
                                  "host": socket.gethostname(), "project_slug": "p", "command": json.dumps(command)}
        Handler.fail_listing = True
        result = self.result("--json", "watch", "cancel", ident[:8])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)["cancelled"])
        self.assertFalse(marker.exists())
        self.assertFalse(any(method == "GET" and "/watches?" in path for method, path, _ in Handler.paths))

    def test_sync_marks_a_legacy_orphan_unknown_without_reexecuting_its_command(self):
        ident = str(uuid.uuid4())
        marker, command = self.marker_command()
        Handler.watches[ident] = {"id": ident, "status": "running", "title": "Orphan",
                                  "host": socket.gethostname(), "project_slug": "p", "command": json.dumps(command)}
        self.cli("sync")
        self.assertEqual(Handler.watches[ident]["status"], "lost")
        self.assertFalse(marker.exists())
        self.assertFalse(self.authorizations())

    def test_cancel_a_queued_start_preserves_both_requests_and_prevents_execution(self):
        Handler.fail_creation = True
        marker, command = self.marker_command()
        result = self.result("--json", "watch", "start", "--title", "Cancel pending", "--", *command)
        created = json.loads(result.stdout)
        result = self.result("--offline", "--json", "watch", "cancel", created["watch_id"][:8])
        self.assertNotEqual(result.returncode, 0)
        cancellation = json.loads(result.stdout)
        self.assertTrue(cancellation["queued"])
        self.assertFalse(cancellation["cancelled"])
        self.assertNotEqual(cancellation["request_id"], created["request_id"])
        self.assertFalse(self.authorizations())
        status = json.loads(self.cli("status", "--json"))
        self.assertEqual(status["pending_outbox"], 2)
        Handler.fail_creation = False
        self.cli("sync")
        self.assertEqual(Handler.watches[created["watch_id"]]["status"], "cancelled")
        self.assertFalse(marker.exists())
