from contextlib import closing
from concurrent.futures import ThreadPoolExecutor
import json
import sqlite3
import time
from delegation_support import DelegationCase


class DelegationContentionTest(DelegationCase):
    def test_supervisor_keeps_the_guest_alive_after_the_database_retry_budget(self):
        self.check_lock(32)

    def test_long_write_lock_does_not_terminate_the_guest_or_lose_its_result(self):
        self.check_lock(6.5)

    def check_lock(self, duration):
        self.env["CJ_DELEGATION_JOURNAL"] = "on"
        session = self.call("start", "claude", "--prompt", "Survive contention", "--stall-timeout", "90", behavior="contention")["session"]
        self.wait_for(session["id"], "ready")
        before = self.call("status", session["id"])["session"]
        database = self.root/"state/codejournal/delegations/delegations.sqlite3"
        with closing(sqlite3.connect(database)) as lock:
            lock.execute("BEGIN IMMEDIATE")
            time.sleep(duration)
            lock.rollback()
        (self.root/"release-guest").touch()
        result = self.settled(session["id"])
        self.assertEqual(result["session"]["status"], "completed", result["session"]["error"])
        self.assertEqual(result["session"]["attempt"], before["attempt"])
        self.assertEqual(result["session"]["result"], "Completed after database contention")
        self.assertFalse(any(e["kind"] == "process_lost" for e in result["events"]))

    def test_concurrent_first_use_preserves_every_session_and_lifecycle_event(self):
        self.env["CJ_DELEGATION_JOURNAL"] = "on"
        with ThreadPoolExecutor(max_workers=12) as pool:
            created = list(pool.map(lambda i: self.call("create", "claude", "--prompt", f"First use {i}"), range(12)))
        self.assertEqual(len({item["session"]["id"] for item in created}), 12)
        self.assertEqual(len(self.call("list")["sessions"]), 12)
        self.assertEqual(len(list((self.root/"state/codejournal/outbox").glob("*.json"))), 12)

    def test_parallel_clients_keep_every_message_and_exactly_one_delivery_file(self):
        self.env["CJ_DELEGATION_JOURNAL"] = "on"
        session = self.call("create", "claude", "--prompt", "Concurrent clients")["session"]

        def publish(index):
            return self.call("publish", session["id"], "--kind", "checkpoint", "--body", f"Concurrent checkpoint {index}", "--idempotency-key", f"checkpoint-{index}")

        with ThreadPoolExecutor(max_workers=12) as pool:
            responses = list(pool.map(publish, range(36)))
        state = self.call("status", session["id"], "--full")
        checkpoints = [event for event in state["events"] if event["kind"] == "checkpoint"]
        self.assertEqual(len(checkpoints), 36)
        self.assertEqual(len({item["event"]["seq"] for item in responses}), 36)
        files = list((self.root/"state/codejournal/outbox").glob("*.json"))
        events = [json.loads(path.read_text()) for path in files]
        self.assertEqual(len(events), 37)
        self.assertEqual(len({event["id"] for event in events}), len(events))
        self.assertFalse(list((self.root/"state/codejournal/outbox").glob(".tmp*")))
