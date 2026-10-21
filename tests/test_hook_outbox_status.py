import json
import subprocess
from auto_sync_support import Handler
from hook_outbox_support import HookOutboxCase


class HookOutboxStatusTest(HookOutboxCase):
    def test_status_explains_every_scope_and_includes_hook_age_without_mutation(self):
        self.save(self.origin())
        self.save(self.origin(token="another-account"))
        self.save()
        path, event = self.save()
        event.pop("origin")
        path.write_text(json.dumps(event))
        self.enqueue()
        before = self.snapshot()
        status = self.cli("--offline", "status")
        self.assertEqual((status["pending_outbox"], status["pending_hook_events"]), (5, 4))
        automatic = status["automatic_sync"]
        self.assertEqual((automatic["pending_current_scope"], automatic["pending_other_scopes"]), (2, 1))
        hooks = automatic["hook_events"]
        self.assertEqual((hooks["current_scope"], hooks["other_scopes"], hooks["unbound"], hooks["unknown_scope"]),
                         (1, 1, 2, 0))
        self.assertEqual(automatic["oldest_pending_at"], 1792275839)
        self.assertIn("have no account binding", " ".join(automatic["hook_notices"]))
        text = subprocess.run([self.binary, "--offline", "status"], cwd=self.base,
                              env=self.env, capture_output=True, text=True, check=True).stdout
        self.assertIn("2 hook event(s) have no account binding", text)
        self.assertIn("cj outbox hooks list", text)
        self.assertFalse(automatic["blocked"])
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(Handler.hook_calls, [])

    def test_unconfigured_status_marks_bound_events_unknown_instead_of_other_accounts(self):
        self.save(self.origin())
        self.save()
        self.config.unlink()
        self.env.pop("CJ_TOKEN")
        status = self.cli("--offline", "status")["automatic_sync"]
        self.assertIsNone(status["pending_current_scope"])
        self.assertIsNone(status["pending_other_scopes"])
        self.assertIsNone(status["hook_events"]["current_scope"])
        self.assertEqual(status["hook_events"]["unknown_scope"], 1)
        self.assertEqual(status["hook_events"]["unbound"], 1)

    def test_disabled_delivery_is_distinct_from_another_account(self):
        self.save(self.origin())
        self.env["CODE_JOURNAL_HOOK_FLUSH"] = "off"
        status = self.cli("--offline", "status")["automatic_sync"]
        self.assertEqual((status["pending_current_scope"], status["pending_other_scopes"]), (0, 0))
        self.assertEqual(status["hook_events"]["current_scope"], 1)
        self.assertFalse(status["hook_events"]["delivery_enabled"])
        self.assertIn("delivery is disabled", " ".join(status["hook_notices"]))

    def test_status_honors_the_explicit_server_override(self):
        self.save(self.origin(server="https://other.example"))
        self.save(self.origin())
        status = self.cli("--offline", "--server", "https://other.example", "status")
        self.assertEqual(status["server"], "https://other.example")
        self.assertEqual(status["automatic_sync"]["hook_events"]["current_scope"], 1)
        self.assertEqual(status["automatic_sync"]["pending_other_scopes"], 1)

    def test_sync_explains_unbound_and_foreign_hooks_without_touching_them(self):
        path, event = self.save(self.origin())
        self.save(self.origin(tenant="other-workspace"))
        self.save()
        before = self.snapshot()
        result = self.cli("sync")
        self.assertEqual(result["flushed"], 1)
        self.assertEqual(result["pending_outbox"], 2)
        self.assertEqual(result["hook_events"]["unbound"], 1)
        self.assertEqual(result["hook_events"]["other_scopes"], 1)
        self.assertIn("cannot sync", " ".join(result["notices"]))
        self.assertEqual(self.snapshot(), {key: value for key, value in before.items() if key != path.name})
        self.assertEqual(Handler.events_applied, {event["id"]})
