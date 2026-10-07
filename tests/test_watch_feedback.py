import json
import pathlib
import socket
import subprocess
import time
import uuid
from watch_support import WatchCase, Handler, ROOT


class WatchFeedbackTest(WatchCase):
    def test_watch_listing_escapes_terminal_controls_and_keeps_json_data(self):
        ident = str(uuid.uuid4())
        title = "Title\x1b]52;c;attacker\x07"
        command = "echo \x1b[2J\x9b31m"
        Handler.watches[ident] = {"id": ident, "status": "running", "title": title,
                                  "command": json.dumps([command]), "host": socket.gethostname()}
        output = self.cli("watch", "list", "--all")
        for character in ["\x1b", "\x07", "\x9b"]:
            self.assertNotIn(character, output)
        self.assertIn("attacker", output)
        structured = json.loads(self.cli("--json", "watch", "list", "--all"))
        self.assertEqual(structured["watches"][0]["title"], title)
        self.assertEqual(json.loads(structured["watches"][0]["command"]), [command])

    def test_worker_rejects_remote_commands_without_local_authorization(self):
        ident = str(uuid.uuid4())
        marker = pathlib.Path(self.temp.name) / "executed"
        Handler.watches[ident] = {"id": ident, "status": "running", "cwd": str(ROOT),
                                  "command": json.dumps(["touch", str(marker)]), "timeout": None}
        result = subprocess.run([self.binary, "--project", "p", "watch", "run", ident],
                                env=self.env, cwd=ROOT, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("local execution authorization", result.stderr)
        self.assertFalse(marker.exists())

    def test_worker_rejects_a_mutated_remote_definition(self):
        marker = pathlib.Path(self.temp.name) / "executed"
        Handler.override = {"command": json.dumps(["touch", str(marker)])}
        result = subprocess.run([self.binary, "--project", "p", "--json", "watch", "start", "--title", "Approved", "--", "true"],
                                env=self.env, cwd=ROOT, capture_output=True, text=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        watch = next(iter(Handler.watches.values()))
        state = pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches"
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline and (list(state.glob("*.authorized.json")) or list(state.glob("*.pid"))):
            time.sleep(0.05)
        self.assertFalse(list(state.glob("*.authorized.json")))
        self.assertFalse(list(state.glob("*.claimed-*")))
        self.assertFalse(list(state.glob("*.pid")))
        self.assertFalse(marker.exists())
        result = subprocess.run([self.binary, "--project", "p", "watch", "run", watch["id"]],
                                env=self.env, cwd=ROOT, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(marker.exists())

    def test_watch_failure_timeout_cancel_and_delivery_message(self):
        failed = json.loads(self.cli("--json", "watch", "start", "--title", "Fake CI", "--",
                                     "sh", "-c", "echo FAILED; exit 3"))["watch"]
        result = self.wait(failed["id"], "finished")
        self.assertEqual(result["exit_code"], 3)
        self.assertIn("FAILED", result["tail"])
        self.assertEqual(Handler.notifications[-1]["title"], "Fake CI: failed")
        quiet = json.loads(self.cli("--json", "watch", "start", "--title", "Quiet", "--notify-on",
                                    "failure", "--", "true"))["watch"]
        self.wait(quiet["id"], "finished")
        self.assertEqual(len(Handler.notifications), 1)
        slow = json.loads(self.cli("--json", "watch", "start", "--title", "Slow", "--timeout", "1",
                                   "--", "sleep", "30"))["watch"]
        self.wait(slow["id"], "timed_out")
        self.assertEqual(Handler.notifications[-1]["title"], "Slow: timed out")
        running = json.loads(self.cli("--json", "watch", "start", "--title", "Long", "--",
                                      "sleep", "30"))["watch"]
        listing = self.cli("watch", "list", "--all")
        self.assertIn("Fake CI", listing)
        self.assertIn("[sh -c echo FAILED; exit 3]", listing)
        other = subprocess.run([self.binary, "--project", "another", "watch", "list", "--all"],
                               env=self.env, cwd=ROOT, capture_output=True, text=True,
                               timeout=5, check=True)
        self.assertIn("Fake CI", other.stdout)
        self.assertIn("Cancelled watch", self.cli("watch", "cancel", running["id"][:8]))
        self.wait(running["id"], "cancelled")
        self.assertEqual(len(Handler.notifications), 2)
        text = self.cli("watch", "start", "--title", "Message", "--", "true")
        self.assertIn("a notification goes to: dashboard bell.", text)
        self.assertIn("desktop and ntfy delivery are off", text)
        message = next(watch for watch in Handler.watches.values() if watch["title"] == "Message")
        self.wait(message["id"], "finished")


if __name__ == "__main__":
    unittest.main()
