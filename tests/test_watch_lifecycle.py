import json
import os
import pathlib
import subprocess
import time
from watch_support import WatchCase, Handler, ROOT


class WatchLifecycleTest(WatchCase):
    def test_runner_has_an_independent_session_and_renews_its_lease(self):
        result = subprocess.run([self.binary, "--project", "p", "--json", "watch", "start", "--title", "Lease", "--", "sleep", "60"],
                                env=self.env, cwd=ROOT, capture_output=True, text=True, timeout=15, start_new_session=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(result.stdout)
        self.assertTrue(data["started"])
        self.assertEqual(data["watch"]["status"], "running")
        self.assertEqual(os.getsid(data["pid"]), data["pid"])
        deadline = time.monotonic() + 34
        while time.monotonic() < deadline and Handler.heartbeats < 2:
            time.sleep(0.05)
        self.assertGreaterEqual(Handler.heartbeats, 2)
        self.cli("watch", "cancel", data["watch"]["id"])
        self.wait(data["watch"]["id"], "cancelled")

    def test_timeout_stops_the_entire_command_process_group(self):
        marker = pathlib.Path(self.temp.name) / "child.pid"
        command = f"sleep 30 & echo $! > '{marker}'; wait"
        data = json.loads(self.cli("--json", "watch", "start", "--title", "Tree", "--timeout", "1", "--", "sh", "-c", command))
        self.wait(data["watch"]["id"], "timed_out")
        pid = int(marker.read_text())
        stat = pathlib.Path(f"/proc/{pid}/stat")
        self.assertTrue(not stat.exists() or stat.read_text().rsplit(")", 1)[1].split()[0] == "Z")

    def test_cancel_does_not_signal_an_unrelated_pid_in_a_stale_record(self):
        data = json.loads(self.cli("--json", "watch", "start", "--title", "Identity", "--", "sleep", "30"))
        ident = data["watch"]["id"]
        path = pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches" / f"{ident}.pid"
        original = json.loads(path.read_text())
        unrelated = subprocess.Popen(["sleep", "30"])
        try:
            path.write_text(json.dumps(dict(original, pid=unrelated.pid)))
            result = json.loads(self.cli("--json", "watch", "cancel", ident))
            self.assertFalse(result["local_process_stopped"])
            self.assertIsNone(unrelated.poll())
        finally:
            unrelated.terminate()
            unrelated.wait(timeout=3)
            path.write_text(json.dumps(original))
            try:
                os.kill(data["pid"], 15)
            except ProcessLookupError:
                pass
