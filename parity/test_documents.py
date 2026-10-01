"""Plans and docs preserve content, history, scheduling and refs."""
from support import ParityCase


class DocumentParityTest(ParityCase):
    def test_plan_and_doc_revisions(self):
        for noun in ["plan", "doc"]:
            with self.subTest(noun=noun):
                created = self.paired(noun, "create", "--title", "Synthetic guide", "--body",
                                      "# Synthetic guide\n\n- [ ] Validate\n", "--ref", "path:README.md")
                for engine, result in created.items():
                    identity = result[noun]["id"]
                    first = self.invoke(engine, noun, "show", identity)
                    self.assertEqual(first[noun]["body"], "- [ ] Validate")
                    self.invoke(engine, noun, "update", identity, "--note", "Review without body")
                    self.invoke(engine, noun, "update", identity, "--base", "2", input="- [x] Validate\n")
                    latest = self.invoke(engine, noun, "show", identity, "--history")
                    self.assertEqual(latest[noun]["revision"], 3)
                    self.assertEqual(latest[noun]["body"], "- [x] Validate")
                    self.assertEqual(sorted(revision["revision"] for revision in latest["revisions"]), [1, 2, 3])
                    historical = self.invoke(engine, noun, "show", identity, "--revision", "1")[noun]
                    self.assertEqual(historical["body"], "- [ ] Validate")
                    self.assertEqual(historical["revision"], 1)
                    self.assertEqual(historical["refs"], [{"kind": "path", "value": "README.md"}])
                path_filter = ("--path", "README.md") if noun == "doc" else ()
                self.titles(self.paired(noun, "list", "--grep", "guide", *path_filter), noun + "s", ["Synthetic guide"])

    def test_plan_schedule_status_and_log(self):
        plans = self.paired("plan", "create", "--title", "Release", "--body", "- [ ] Ship", "--status", "active")
        for engine, result in plans.items():
            identity = result["plan"]["id"]
            self.invoke(engine, "plan", "schedule", identity, "2099-01-02")
            self.assertEqual(self.invoke(engine, "plan", "show", identity)["plan"]["not_before"], "2099-01-02")
            self.invoke(engine, "log", "add", "--title", "Prepared release", "--body", "Synthetic work",
                        "--plan", identity, "--no-auto-commits")
            self.assertEqual(self.invoke(engine, "log", "list", "--plan", identity)["logs"][0]["title"], "Prepared release")
            self.invoke(engine, "plan", "status", identity, "done", "--note", "Shipped")
        self.titles(self.paired("plan", "list"), "plans", [])
        self.titles(self.paired("plan", "list", "--status", "done"), "plans", ["Release"])

    def test_body_roundtrip_and_invalid_revision(self):
        docs = self.paired("doc", "create", "--title", "Runbook", "--body", "One paragraph.")
        for engine, result in docs.items():
            identity = result["doc"]["id"]
            body = self.invoke(engine, "doc", "show", identity, "--body", json_mode=False).stdout
            self.invoke(engine, "doc", "update", identity, input=body)
            self.assertEqual(self.invoke(engine, "doc", "show", identity)["doc"]["body"], "One paragraph.")
            self.assertNotEqual(self.invoke(engine, "doc", "show", identity, "--revision", "99", check=False).returncode, 0)
