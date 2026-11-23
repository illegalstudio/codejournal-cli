import time
import unittest
from hook_outbox_support import HookOutboxCase

HOUR = 3_600_000_000_000


class HookOutboxExpiryTest(HookOutboxCase):
    def queued(self, hours_ago, origin=None):
        path, event = self.save(origin)
        dated = self.folder / f"{time.time_ns() - hours_ago * HOUR:020}-{event['id']}.json"
        path.rename(dated)
        return dated

    def test_sync_removes_only_unbound_events_older_than_two_days(self):
        stale = self.queued(49)
        recent = self.queued(47)
        foreign = self.queued(500, self.origin(tenant="other-workspace"))
        undated, _ = self.save()
        result = self.cli("sync")
        self.assertFalse(stale.exists())
        self.assertTrue(all(path.exists() for path in (recent, foreign, undated)))
        self.assertEqual(result["unbound_hooks_removed"], 1)
        self.assertEqual(result["hook_events"]["unbound"], 2)
        notices = " ".join(result["notices"])
        self.assertIn("Removed 1 hook event(s) that had no account binding for more than 48 hours", notices)
        self.assertIn("removed automatically 48 hours after they were queued", notices)
        again = self.cli("sync")
        self.assertEqual(again["unbound_hooks_removed"], 0)
        self.assertNotIn("Removed", " ".join(again["notices"]))


if __name__ == "__main__":
    unittest.main()
