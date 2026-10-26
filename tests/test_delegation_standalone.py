import json
from delegation_support import DelegationCase


class StandaloneDelegationTest(DelegationCase):
    def test_kimi_model_discovery_includes_aliases_without_provider_configuration(self):
        result=self.call("models","kimi")
        self.assertEqual(result["configured_aliases"][0]["alias"],"configured-model")
        self.assertNotIn("must-not-be-exposed",json.dumps(result))

    def test_binary_alone_runs_all_six_providers_without_python_or_ai_on_path(self):
        self.assertEqual(self.env["PATH"],str(self.bin))
        for target in ("claude","codex","cursor","grok","kimi","pi"):
            with self.subTest(target=target):
                delegation_id=self.start(target=target)
                result=self.settled(delegation_id)
                self.assertEqual(result["session"]["status"],"completed")
                self.assertEqual(result["session"]["result"],"Standalone provider completed")
                self.assertNotIn("attempt_token",result["session"])
                self.assertNotIn("journal_context",result["session"])
        self.assertEqual(len(self.call("list","--all")["sessions"]),6)
        self.assertEqual(self.call("list")["sessions"],[])

    def test_question_answer_ack_and_authoritative_result_survive_cli_exits(self):
        delegation_id=self.start("question")
        question=self.wait_for(delegation_id,"question")
        self.call("ack",delegation_id,"--through",str(question["seq"]))
        unread=self.call("sync",delegation_id)
        self.assertEqual(unread["events"],[])
        self.call("answer",delegation_id,"--reply-to",str(question["seq"]),"--body","Approved bounded answer")
        result=self.settled(delegation_id)
        self.assertEqual(result["session"]["status"],"completed")
        self.assertIn("host answer",result["session"]["result"])
        self.assertEqual(self.call("status",delegation_id)["session"]["id"],delegation_id)

    def test_plain_output_never_claims_success(self):
        delegation_id=self.start("noresult")
        result=self.settled(delegation_id)
        self.assertEqual(result["session"]["status"],"stalled")
        self.assertIn("without publishing a result",result["session"]["error"])
        exited=self.wait_for(delegation_id,"process_exit")
        self.assertIn("diagnostic",exited["payload"]["stdout_tail"])

    def test_mcp_is_embedded_and_completes_without_external_server(self):
        for target in ("claude","codex"):
            with self.subTest(target=target):
                delegation_id=self.start("mcp",target=target)
                result=self.settled(delegation_id)
                self.assertEqual(result["session"]["result"],"Completed through embedded MCP")

    def test_guest_cannot_impersonate_host_or_launch_nested_agents(self):
        delegation_id=self.start("impersonate")
        result=self.settled(delegation_id)
        self.assertEqual(result["session"]["status"],"completed")
        self.assertIn("rejected",result["session"]["result"])

    def test_guest_hooks_link_the_session_using_native_context(self):
        repository=self.repository()
        delegation_id=self.start("hooks",cwd=repository)
        result=self.settled(delegation_id)
        self.assertEqual(result["session"]["status"],"completed")
        events=[json.loads(path.read_text()) for path in (self.root/"state/codejournal/outbox").glob("*.json")]
        guest=next(event for event in events if event["type"]=="guest")
        self.assertEqual(guest["delegation_id"],delegation_id)

    def test_runtime_directory_changes_do_not_split_the_broker(self):
        created=self.call("create","claude","--prompt","Persist a task")["session"]
        other=self.env.copy()
        other["XDG_RUNTIME_DIR"]=str(self.root/"runtime-b")
        read=self.call("status",created["id"],"--full",env=other)
        self.assertEqual(read["session"]["task"],"Persist a task")
        self.assertEqual(len(self.call("list",env=other)["sessions"]),1)

    def test_dry_run_does_not_launch_or_create_state(self):
        result=self.call("start","codex","--prompt","- [ ] Inspect","--dry-run")
        self.assertTrue(result["dry_run"])
        self.assertIn("exec",result["args"])
        self.assertFalse((self.root/"state/codejournal/delegations/delegations.sqlite3").exists())

    def test_cancel_stops_the_owned_guest_and_keeps_durable_history(self):
        delegation_id=self.start("sleep")
        self.call("cancel",delegation_id,"--reason","End this synthetic run")
        result=self.settled(delegation_id)
        self.assertEqual(result["session"]["status"],"cancelled")
        self.assertIsNone(result["session"]["guest_pid"])
        self.assertIn("End this synthetic run",result["session"]["error"])
