from contextlib import closing
import os
import signal
import sqlite3
import sys
import time
import unittest
from pathlib import Path
from delegation_support import DelegationCase


class DelegationProcessOwnershipTest(DelegationCase):
    def update(self, delegation_id, changes, values):
        database = self.root/"state/codejournal/delegations/delegations.sqlite3"
        with closing(sqlite3.connect(database)) as db, db:
            db.execute("UPDATE delegations SET "+changes+" WHERE id=?", (*values, delegation_id))

    def test_terminal_writer_retains_ownership_until_its_process_stops(self):
        repository = self.repository()
        session = self.call("create", "claude", "--mode", "write", "--prompt", "Edit", cwd=repository)["session"]
        self.update(session["id"], "status='cancelled',guest_pid=?", (os.getpid(),))
        other = self.repository("other")
        blocked = self.invoke("create", "claude", "--mode", "write", "--prompt", "Edit", cwd=other, check=False)
        self.update(session["id"], "guest_pid=NULL", ())
        self.assertNotEqual(blocked.returncode, 0)
        self.assertIn("another write delegation", blocked.stderr)
        self.call("create", "claude", "--mode", "write", "--prompt", "Edit", cwd=other)

    def test_reused_supervisor_pid_cannot_block_recovery(self):
        session = self.call("create", "claude", "--prompt", "Recover safely")["session"]
        self.update(session["id"], "status='stalled',attempt=1,supervisor_pid=?,supervisor_identity=?", (os.getpid(), "different-process-start"))
        recovered = self.invoke("recover", session["id"], behavior="finish", check=False)
        if recovered.returncode:
            self.update(session["id"], "supervisor_pid=NULL,supervisor_identity=NULL", ())
        self.assertEqual(recovered.returncode, 0, recovered.stderr)
        self.assertEqual(self.settled(session["id"])["session"]["status"], "completed")

    @unittest.skipUnless(sys.platform == "linux", "checks native Linux process start identity")
    def test_reused_supervisor_pid_cannot_hide_an_owned_orphan_from_cancel(self):
        delegation_id = self.start("sleep")
        self.wait_for(delegation_id, "ready")
        session = self.call("status", delegation_id)["session"]
        os.kill(session["supervisor_pid"], signal.SIGKILL)
        self.update(delegation_id, "supervisor_pid=?,supervisor_identity=?", (os.getpid(), "different-process-start"))
        self.call("cancel", delegation_id)
        deadline = time.monotonic()+3
        guest = Path(f'/proc/{session["guest_pid"]}/stat')
        while time.monotonic() < deadline:
            try:
                if guest.read_text().rsplit(")", 1)[1].split()[0] in ("Z", "X"):
                    break
            except FileNotFoundError:
                break
            time.sleep(.04)
        else:
            os.killpg(session["guest_pid"], signal.SIGTERM)
            self.fail("Cancellation left the confirmed guest running behind a reused supervisor PID")
