from contextlib import closing
import json
import os
import sqlite3
import subprocess
import time
from delegation_support import DelegationCase


class DelegationRecoveryTest(DelegationCase):
    def database(self):
        return self.root/"state/codejournal/delegations/delegations.sqlite3"

    def test_recovery_keeps_identity_and_checkpoints_and_fences_the_old_token(self):
        delegation_id=self.start("recover")
        first=self.settled(delegation_id)
        self.assertEqual(first["session"]["status"],"stalled")
        with closing(sqlite3.connect(self.database())) as db, db:
            token=db.execute("SELECT attempt_token FROM delegations WHERE id=?",(delegation_id,)).fetchone()[0]
        self.call("recover",delegation_id,behavior="recover")
        old=self.env.copy()
        old.update({"CJ_DELEGATION_ID":delegation_id,"CJ_DELEGATION_ROLE":"guest","CJ_DELEGATION_TOKEN":token})
        rejected=self.invoke("publish",delegation_id,"--role","guest","--kind","result","--body","Late old attempt",env=old,check=False)
        self.assertNotEqual(rejected.returncode,0)
        self.assertIn("no longer authorized",rejected.stderr)
        recovered=self.settled(delegation_id)
        self.assertEqual(recovered["session"]["attempt"],2)
        self.assertEqual(recovered["session"]["status"],"completed")
        self.assertIn("durable checkpoint",recovered["session"]["result"])
        self.assertTrue(any(e["kind"]=="checkpoint" for e in recovered["events"]))

    def test_orphan_detection_preserves_live_guest_and_blocks_duplicate_recovery(self):
        created=self.call("create","claude","--prompt","Synthetic orphan")["session"]
        guest=subprocess.Popen(["/bin/sleep","10"])
        self.addCleanup(guest.wait)
        self.addCleanup(guest.terminate)
        with closing(sqlite3.connect(self.database())) as db, db:
            db.execute("UPDATE delegations SET status='running', attempt=1, supervisor_pid=NULL, guest_pid=?, guest_heartbeat=?,updated_at=? WHERE id=?",
                (guest.pid,time.time()-60,time.time(),created["id"]))
        first=self.call("status",created["id"])
        second=self.call("status",created["id"])
        self.assertEqual(first["session"]["status"],"stalled")
        self.assertEqual(first["session"]["guest_pid"],guest.pid)
        self.assertEqual(sum(e["kind"]=="process_lost" for e in second["events"]),1)
        rejected=self.invoke("recover",created["id"],check=False)
        self.assertNotEqual(rejected.returncode,0)
        self.assertIn("live process",rejected.stderr)

    def test_idempotent_messages_do_not_duplicate_history(self):
        created=self.call("create","claude","--prompt","Message retries")["session"]
        for _ in range(2):
            self.call("send",created["id"],"--body","One message","--idempotency-key","stable-retry")
        events=self.call("status",created["id"])["events"]
        self.assertEqual(sum(e["kind"]=="message" for e in events),1)

    def test_guest_attempt_cannot_read_another_session(self):
        session=self.call("create","claude","--prompt","One")["session"]
        other=self.call("create","claude","--prompt","Two")["session"]
        environment=self.env.copy()
        environment.update({"CJ_DELEGATION_ID":session["id"],"CJ_DELEGATION_ROLE":"guest","CJ_DELEGATION_TOKEN":"invalid"})
        rejected=self.invoke("status",other["id"],"--role","guest",env=environment,check=False)
        self.assertNotEqual(rejected.returncode,0)
        self.assertIn("no longer authorized",rejected.stderr)

    def test_stall_timeout_is_not_extended_by_supervisor_heartbeats(self):
        session=self.call("start","claude","--prompt","Sleep","--stall-timeout","0.1",behavior="sleep")["session"]
        result=self.settled(session["id"])
        self.assertEqual(result["session"]["status"],"stalled")
        self.assertIn("no activity",result["session"]["error"])
