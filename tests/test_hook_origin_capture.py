import json
from hook_outbox_support import HookOutboxCase


class HookOriginCaptureTest(HookOutboxCase):
    def test_missing_configuration_is_recorded_without_losing_the_event(self):
        self.env["CODE_JOURNAL_HOOK_FLUSH"] = "off"
        self.config.unlink()
        event = self.capture()
        self.assertIsNone(event["origin"])
        self.assertEqual(event["origin_unavailable_reason"], "configuration_unavailable")

    def test_unavailable_credentials_are_distinct_and_do_not_store_raw_errors(self):
        self.env.update(CODE_JOURNAL_HOOK_FLUSH="off", CJ_SERVER_URL="https://other.example")
        self.env.pop("CJ_TOKEN")
        event = self.capture()
        self.assertIsNone(event["origin"])
        self.assertEqual(event["origin_unavailable_reason"], "credentials_unavailable")
        self.assertNotIn("stored token", json.dumps(event))

    def test_successful_capture_records_the_exact_credential_scope(self):
        self.env["CODE_JOURNAL_HOOK_FLUSH"] = "off"
        event = self.capture()
        self.assertEqual(event["origin"], self.origin())
        self.assertNotIn("origin_unavailable_reason", event)
