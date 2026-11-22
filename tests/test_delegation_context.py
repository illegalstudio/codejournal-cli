import hashlib
import json
from delegation_support import DelegationCase


READ_CONTEXT = r'''
if os.environ.get("CJ_FAKE_BEHAVIOR") == "context":
    import hashlib
    for item in call("context", delegation_id)["attachments"]:
        offset = 0
        pieces = []
        while True:
            part = call("context", delegation_id, "--name", item["name"], "--offset", str(offset), "--limit", "12000")["attachment"]
            pieces.append(part["body"])
            if part["next_offset"] is None:
                break
            offset = part["next_offset"]
        if hashlib.sha256("".join(pieces).encode()).hexdigest() != item["sha256"]:
            raise RuntimeError("Context snapshot hash mismatch")
'''


GH = r'''
import json, os, sys
args = sys.argv[1:]
with open(os.environ["CJ_FAKE_GH_LOG"], "a") as log:
    log.write(json.dumps(args) + "\n")
record = {"title": "Fix lowering", "state": "OPEN", "author": {"login": "alice"}, "body": "Why it changed",
    "comments": [{"author": {"login": "bob"}, "createdAt": "2026-11-21T10:00:00Z", "body": "Please test"}]}
if args[:3] == ["pr", "view", "7"]:
    print(json.dumps({**record, "number": 7, "url": "https://github.test/owner/repo/pull/7",
        "headRefName": "fix", "baseRefName": "main", "reviews": []}))
elif args[:3] == ["pr", "diff", "7"]:
    print("diff --git a/x b/x\n+new line")
elif args[:3] == ["issue", "view", "9"]:
    print(json.dumps({**record, "number": 9, "title": "Strict mode fails", "url": "https://github.test/owner/repo/issues/9"}))
else:
    sys.stderr.write("HTTP 401: Bad credentials\n")
    sys.exit(1)
'''


class DelegationContextTest(DelegationCase):
    def setUp(self):
        super().setUp()
        fake = self.bin / "claude"
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")', 'publish("ready", "Protocol loaded")\n' + READ_CONTEXT))

    def test_immutable_context_survives_file_changes_and_records_read_ranges(self):
        source = self.root / "ledger.txt"
        content = "immutable evidence €\n" * 6000
        source.write_text(content)
        ident = self.call("create", "claude", "--prompt", "Read the attached evidence", "--context-file", str(source))["id"]
        source.write_text("changed after capture")
        self.call("recover", ident, behavior="context")
        snapshot = self.settled(ident)
        self.assertEqual(snapshot["status"], "completed", json.dumps(snapshot))
        reads = [event["payload"] for event in snapshot["events"] if event["kind"] == "context_read"]
        self.assertGreater(len(reads), 1)
        self.assertEqual(reads[0]["sha256"], hashlib.sha256(content.encode()).hexdigest())
        self.assertEqual(reads[0]["offset"], 0)
        self.assertEqual(reads[-1]["end"], len(content))
        recovered = self.call("context", ident, "--name", "file-1", "--limit", "20")
        self.assertEqual(recovered["attachment"]["body"], content[:20])

    def test_result_is_blocked_until_required_context_has_been_read(self):
        source = self.root / "ledger.txt"
        source.write_text("Required evidence")
        ident = self.call("start", "claude", "--prompt", "Read the evidence", "--context-file", str(source))["id"]
        snapshot = self.settled(ident)
        self.assertEqual(snapshot["status"], "stalled")
        self.assertIn("unread ranges", snapshot["session"]["error"])

    def test_previous_result_can_be_attached_without_cross_session_access(self):
        previous = self.start()
        self.settled(previous)
        ident = self.call("start", "claude", "--prompt", "Recheck the prior result", "--context-result", previous, behavior="context")["id"]
        self.assertEqual(self.settled(ident)["status"], "completed")
        part = self.call("context", ident, "--name", "result-1")["attachment"]
        self.assertEqual(part["source"]["delegation_id"], previous)
        self.assertEqual(part["body"], "Standalone provider completed")

    def test_github_records_are_read_by_the_host_and_attached_for_the_guest(self):
        import sys
        gh = self.bin / "gh"
        gh.write_text(f"#!{sys.executable}\n" + GH)
        gh.chmod(0o755)
        log = self.root / "gh.log"
        self.env["CJ_FAKE_GH_LOG"] = str(log)
        ident = self.call("create", "claude", "--prompt", "Review the pull request",
            "--context-github", "owner/repo#7", "--context-github", "#9")["id"]
        calls = [json.loads(line) for line in log.read_text().splitlines()]
        self.assertEqual(calls[0][:5], ["pr", "view", "7", "--repo", "owner/repo"])
        self.assertEqual(calls[1], ["pr", "diff", "7", "--repo", "owner/repo"])
        self.assertEqual([call[:3] for call in calls[2:]], [["pr", "view", "9"], ["issue", "view", "9"]])
        manifest = self.call("context", ident)["attachments"]
        self.assertEqual([item["name"] for item in manifest], ["github-1", "github-2"])
        self.assertEqual(manifest[0]["source"], {"github": "owner/repo#7", "url": "https://github.test/owner/repo/pull/7"})
        pull = self.call("context", ident, "--name", "github-1", "--limit", "12000")["attachment"]["body"]
        for expected in ["# Pull request #7: Fix lowering", "Branches: fix into main", "### bob at 2026-11-21T10:00:00Z",
                         "Please test", "## Diff", "+new line"]:
            self.assertIn(expected, pull)
        issue = self.call("context", ident, "--name", "github-2", "--limit", "12000")["attachment"]["body"]
        self.assertTrue(issue.startswith("# Issue #9: Strict mode fails"), issue)
        self.assertNotIn("## Diff", issue)
        for reference, message in [("#404", "could not read #404 as a pull request"), ("--web", "invalid GitHub reference")]:
            failed = self.invoke("create", "claude", "--prompt", "Review", f"--context-github={reference}", check=False)
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn(message, failed.stderr)
        self.assertEqual(len(self.call("list", "--all")["sessions"]), 1)

    def test_large_unicode_task_is_loaded_from_canonical_state(self):
        prompt = self.root / "task.txt"
        prompt.write_text("task €" * 16000)
        ident = self.call("start", "claude", "--prompt-file", str(prompt))["id"]
        self.assertEqual(self.settled(ident)["status"], "completed")
        self.assertEqual(self.call("sync", ident, "--full")["session"]["task"], prompt.read_text())
