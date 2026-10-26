import json
import subprocess
import time
import uuid
from auto_sync_support import AutoSyncCase, Handler


class AutoSyncTest(AutoSyncCase):
    def test_new_online_writes_join_the_queue_instead_of_overtaking_it(self):
        first = self.enqueue("First")
        second = self.cli("add", "--kind", "discovery", "--title", "Second", "--body", "Synthetic")
        self.assertTrue(second["queued"])
        self.wait_for(lambda: not self.queue())
        self.assertEqual([call[0] for call in Handler.calls], [first["id"], second["id"]])
        self.assertEqual([body["title"] for _, body in Handler.applied], ["First", "Second"])

    def test_uncertain_write_recovers_without_another_command_and_does_not_duplicate(self):
        Handler.failures[:] = [503, 503]
        started = time.monotonic()
        queued = self.cli("add", "--kind", "discovery", "--title", "Kept note", "--body", "Synthetic")
        self.assertLess(time.monotonic() - started, 2)
        self.wait_for(lambda: self.saved().get("next_retry_at") is not None)
        self.assertTrue(self.queue())
        self.wait_for(lambda: not self.queue())
        self.assertEqual([call[0] for call in Handler.calls], [queued["id"]] * 3)
        self.assertEqual(len(Handler.applied), 1)
        self.assertGreaterEqual(Handler.calls[2][3] - Handler.calls[1][3], 4)
        status = self.cli("status")
        self.assertEqual(status["pending_writes"], 0)
        self.assertEqual(status["pending_hook_events"], 0)
        self.assertIsNotNone(status["last_sync_at"])

    def test_online_command_wakes_offline_queue_and_order_survives_batches(self):
        first = self.enqueue("0")
        template = json.loads(self.queue()[0].read_text())
        directory = self.queue()[0].parent
        self.queue()[0].unlink()
        ids = [first["id"]]
        for i in range(51):
            item = dict(template, id=first["id"] if i == 0 else str(uuid.uuid4()))
            item["body"] = dict(template["body"], title=str(i))
            (directory / f"{i:020}-{item['id']}.json").write_text(json.dumps(item))
            if i:
                ids.append(item["id"])
        self.assertEqual(Handler.calls, [])
        self.cli("hook-flush")
        self.wait_for(lambda: not self.queue())
        self.assertEqual([call[0] for call in Handler.calls], ids)
        self.assertEqual([body["title"] for _, body in Handler.applied], [str(i) for i in range(51)])

    def test_parallel_wakeups_have_one_delivery_owner(self):
        queued = self.enqueue()
        processes = [subprocess.Popen([self.binary, "--project", "fixture", "projects"],
            cwd=self.base, env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) for _ in range(8)]
        for process in processes:
            self.assertEqual(process.communicate(timeout=10)[1], b"")
            self.assertEqual(process.returncode, 0)
        self.wait_for(lambda: not self.queue())
        self.assertEqual([call[0] for call in Handler.calls], [queued["id"]])

    def test_initial_rate_limit_waits_for_retry_after(self):
        Handler.failures[:] = [429]
        Handler.retry_after = 2
        queued = self.cli("add", "--kind", "discovery", "--title", "Rate limited", "--body", "Synthetic")
        self.wait_for(lambda: self.saved().get("next_retry_at") is not None)
        self.assertEqual(len(Handler.calls), 1)
        self.wait_for(lambda: not self.queue())
        self.assertEqual([call[0] for call in Handler.calls], [queued["id"]] * 2)
        self.assertGreaterEqual(Handler.calls[1][3] - Handler.calls[0][3], 2)

    def test_retry_state_survives_worker_restart_and_retains_original_request(self):
        Handler.failures[:] = [503]
        queued = self.enqueue()
        self.cli("projects")
        self.wait_for(lambda: self.saved().get("next_retry_at") is not None)
        saved = self.saved()
        data = self.config.read_text()
        self.config.unlink()
        time.sleep(5.1)
        self.config.write_text(data)
        self.cli("projects")
        self.wait_for(lambda: not self.queue())
        self.assertEqual([call[0] for call in Handler.calls], [queued["id"]] * 2)
        self.assertGreaterEqual(Handler.calls[1][3] - Handler.calls[0][3], 4)
        self.assertIsNotNone(saved["last_error"])
