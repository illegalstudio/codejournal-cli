import json
import os
from delegation_support import DelegationCase


LARGE_OUTPUT = r'''
if os.environ.get("CJ_FAKE_BEHAVIOR") == "large_output":
    print(json.dumps({"type":"tool_result", "name":"Read", "rawOutput":{"stdout":[65]*900000}}), flush=True)
    print(json.dumps({"type":"tool_call", "name":"NextTool", "text":"LATEST_PROGRESS"}), flush=True)
    for noise in ("thinking_delta", "signature_delta", "input_json_delta"):
        for _ in range(50):
            print(json.dumps({"type":"content_block_delta", "delta":{"type":noise,"signature":"unused"}}), flush=True)
'''


class DelegationRecentOutputTest(DelegationCase):
    def test_capture_keeps_recent_progress_after_the_two_mib_limit(self):
        fake = self.bin / "grok"
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")', 'publish("ready", "Protocol loaded")\n' + LARGE_OUTPUT))
        ident = self.start("large_output", target="grok")
        self.settled(ident)
        result = self.call("output", ident, "--lines", "12", "--max-chars", "512")
        stream = result["streams"]["stdout"]
        self.assertIn("LATEST_PROGRESS", stream["text"])
        self.assertIn("NextTool", stream["text"])
        self.assertNotIn("65, 65, 65", stream["text"])
        self.assertNotIn("signature_delta", stream["text"])
        self.assertNotIn("input_json_delta", stream["text"])
        self.assertEqual(stream["format"], "summary")
        self.assertLessEqual(len(stream["text"]), 512)
        self.assertTrue(stream["capture_truncated"])
        raw = self.call("output", ident, "--raw", "--max-chars", "256")
        self.assertIn("input_json_delta", raw["streams"]["stdout"]["text"])
        self.assertLessEqual(len(raw["streams"]["stdout"]["text"]), 256)
        files = list((self.root / "state/codejournal/delegations").glob("*.log"))
        self.assertTrue(all(file.stat().st_size <= 2097152 for file in files))

    def test_full_and_compact_json_share_status_and_support_events_only(self):
        ident = self.start()
        self.settled(ident)
        compact = self.call("sync", ident)
        full = self.call("sync", ident, "--full", "--events-only")
        self.assertEqual(compact["status"], full["status"])
        self.assertEqual(compact["id"], full["id"])
        self.assertNotIn("task", full["session"])
        self.assertTrue(full["session"]["task_omitted"])
        self.assertIn("task", self.call("sync", ident, "--full")["session"])

    def test_terminal_provider_error_is_available_without_raw_output(self):
        fake = self.bin / "kimi"
        fake.write_text(f'#!{os.sys.executable}\nimport sys, os\nprint(os.environ["CJ_DELEGATION_TOKEN"], flush=True)\nprint("provider.rate_limit:429 The engine is currently overloaded",file=sys.stderr)\nsys.exit(1)\n')
        ident = self.start(target="kimi")
        result = self.settled(ident)
        self.assertIn("engine is currently overloaded", result["session"]["error"])
        event = next(event for event in result["events"] if event["kind"] == "process_exit")
        self.assertIn("provider.rate_limit:429", event["payload"]["provider_error"])
        self.assertIn("[redacted]", event["payload"]["stdout_tail"])
