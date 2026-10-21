from contextlib import closing
import hashlib
import sqlite3
import time
from delegation_support import DelegationCase


class DelegationMigrationTest(DelegationCase):
    def test_import_cannot_modify_the_source_through_a_native_store_alias(self):
        self.call("list")
        source=self.root/"state/codejournal/delegations/delegations.sqlite3"
        original=self.root/"alias.sqlite3"
        original.hardlink_to(source)
        before=hashlib.sha256(source.read_bytes()).hexdigest()
        result=self.invoke("migrate","--from",str(original),check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn("alias",result.stderr)
        self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(),before)

    def test_import_preserves_original_history_and_cannot_overwrite_new_sessions(self):
        session=self.call("create","claude","--prompt","Legacy task")["session"]
        self.call("send",session["id"],"--body","Durable checkpoint")
        source=self.root/"state/codejournal/delegations/delegations.sqlite3"
        original=self.root/"legacy.sqlite3"
        with closing(sqlite3.connect(source)) as db, closing(sqlite3.connect(original)) as backup:
            db.backup(backup)
        with closing(sqlite3.connect(original)) as backup, backup:
            backup.execute("UPDATE delegations SET guest_pid=99999999")
        before=hashlib.sha256(original.read_bytes()).hexdigest()
        # Use another state home to represent a fresh native installation.
        environment=self.env.copy()
        environment["XDG_STATE_HOME"]=str(self.root/"new-state")
        imported=self.call("migrate","--from",str(original),env=environment)
        self.assertEqual(imported["imported_sessions"],1)
        read=self.call("status",session["id"],env=environment)
        self.assertEqual(read["session"]["task"],"Legacy task")
        self.assertIsNone(read["session"]["guest_pid"])
        self.assertTrue(any(e["kind"]=="message" for e in read["events"]))
        self.assertEqual(hashlib.sha256(original.read_bytes()).hexdigest(),before)
        self.assertTrue(self.call("migrate","--from",str(original),env=environment)["already_imported"])
        rejected=self.invoke("migrate","--from",str(original),check=False)
        self.assertNotEqual(rejected.returncode,0)
        self.assertIn("empty delegation store",rejected.stderr)

    def test_import_refuses_live_legacy_processes(self):
        import os
        self.call("create","claude","--prompt","Preserve live guest")
        source=self.root/"state/codejournal/delegations/delegations.sqlite3"
        with closing(sqlite3.connect(source)) as db, db:
            db.execute("UPDATE delegations SET status='running',guest_pid=?,updated_at=?",(os.getpid(),time.time()))
        environment=self.env.copy()
        environment["XDG_STATE_HOME"]=str(self.root/"new-state")
        result=self.invoke("migrate","--from",str(source),env=environment,check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn("live process",result.stderr)
        with closing(sqlite3.connect(source)) as db, db:
            db.execute("UPDATE delegations SET guest_pid=NULL,status='created'")
