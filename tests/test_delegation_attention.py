import json
from delegation_support import DelegationCase


ATTENTION = r'''
if os.environ.get("CJ_FAKE_BEHAVIOR") == "attention":
    if "--settings" in sys.argv:
        settings = json.loads(sys.argv[sys.argv.index("--settings")+1])
    else:
        root = Path(os.environ.get("GROK_HOME", str(Path.home()/".grok")))
        settings = json.loads(next((root/"hooks").glob("cj-delegation-*.json")).read_text())
    hook = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
    foreign = os.environ.copy()
    foreign["CJ_DELEGATION_ID"] = "00000000-0000-4000-8000-000000000000"
    ignored = subprocess.run(hook, shell=True, input="{}", env=foreign, text=True, capture_output=True, check=True)
    if json.loads(ignored.stdout) != {}:
        raise RuntimeError("A hook exposed another delegation's context")
    deadline = time.monotonic()+8
    while time.monotonic() < deadline:
        value = subprocess.run(hook, shell=True, input="{}", text=True, capture_output=True, check=True)
        message = json.loads(value.stdout).get("hookSpecificOutput", {}).get("additionalContext", "")
        if message:
            publish("checkpoint", "Tool boundary received: " + message)
            break
        time.sleep(.04)
    else:
        raise RuntimeError("Host correction did not reach the tool boundary")
'''


class DelegationAttentionTest(DelegationCase):
    def test_claude_tool_boundary_surfaces_host_corrections_before_explicit_sync(self):
        self.exercise("claude")

    def test_grok_tool_boundary_uses_a_private_owned_hook_and_cleans_it_up(self):
        self.env["GROK_HOME"] = str(self.root / "grok-state")
        self.exercise("grok")
        self.assertFalse(list((self.root / "grok-state/hooks").glob("cj-delegation-*.json")))

    def exercise(self, target):
        fake = self.bin / target
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")', 'publish("ready", "Protocol loaded")\n' + ATTENTION))
        ident = self.start("attention", target=target)
        self.wait_for(ident, "ready")
        self.assertEqual(self.call("attention", ident), {})
        self.call("send", ident, "--body", "Inspect the exact immutable SHA before continuing")
        checkpoint = self.wait_for(ident, "checkpoint")
        self.assertIn("exact immutable SHA", checkpoint["payload"]["text"])
        result = self.settled(ident)
        self.assertEqual(result["status"], "completed", json.dumps(result))
        notices = [event for event in result["events"] if event["kind"] == "host_attention"]
        self.assertEqual(len(notices), 1)
        self.assertIsNotNone(result["host_messages"]["last_tool_boundary_notice_at"])
        self.assertFalse(result["host_messages"]["unread"])
