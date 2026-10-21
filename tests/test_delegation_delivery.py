from contextlib import closing
import json
import sqlite3
from pathlib import Path
from delegation_support import DelegationCase


class DelegationDeliveryTest(DelegationCase):
    def configure(self, token):
        path=self.root/"config/codejournal/config.json"
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text(json.dumps({"server":"http://127.0.0.1:9","tenant":"synthetic","fallback_token":token}))
        self.env["CJ_TOKEN"]=token
        self.env["CJ_DELEGATION_JOURNAL"]="on"

    def queued(self):
        return [json.loads(path.read_text()) for path in sorted((self.root/"state/codejournal/outbox").glob("*.json"))]

    def test_lifecycle_is_queued_locally_under_the_original_account(self):
        self.configure("synthetic-original-account")
        session=self.start("question")
        question=self.wait_for(session,"question")
        created=next(event for event in self.queued() if event["kind"]=="created")
        self.configure("synthetic-replacement-account")
        self.call("answer",session,"--reply-to",str(question["seq"]),"--body","Approved")
        self.settled(session)
        events=self.queued()
        kinds=[event["kind"] for event in events]
        for kind in ("created","started","question","answer","result","exited"):
            self.assertIn(kind,kinds)
        self.assertTrue(all(event["origin"]==created["origin"] for event in events))
        result=next(event for event in events if event["kind"]=="result")
        self.assertEqual(result["delegation"]["status"],"completed")
        self.assertIsInstance(result["delegation"]["created_at"],str)
        self.assertNotIn("synthetic-original-account",json.dumps(events))
        self.assertNotIn("attempt_token",json.dumps(events))

    def test_missing_credentials_do_not_prevent_delegation_or_infer_an_account(self):
        self.env["CJ_DELEGATION_JOURNAL"]="on"
        session=self.start()
        result=self.settled(session)
        self.assertEqual(result["session"]["status"],"completed")
        self.assertTrue(self.queued())
        self.assertTrue(all(event["origin"] is None for event in self.queued()))

    def test_durable_delivery_retries_keep_the_same_event_id_and_bytes(self):
        self.configure("synthetic-original-account")
        created=self.call("create","claude","--prompt","Pending delivery")["session"]
        before=self.queued()
        event=before[0]
        file=next(path for path in (self.root/"state/codejournal/outbox").glob("*.json") if event["id"] in path.name)
        data=file.read_bytes()
        epoch=int(file.name.split("-",1)[0])
        database=self.root/"state/codejournal/delegations/delegations.sqlite3"
        with closing(sqlite3.connect(database)) as db, db:
            db.execute("INSERT INTO journal_delivery (id,payload,queued_at) VALUES (?,?,?)",(event["id"],json.dumps(event),str(epoch)))
        self.call("status",created["id"])
        self.assertEqual(file.read_bytes(),data)
        self.assertEqual(len(self.queued()),1)
