from delegation_support import DelegationCase


class DelegationWriteTest(DelegationCase):
    def create_writer(self, repository):
        return self.call("create","claude","--mode","write","--prompt","Change seed.txt",cwd=repository)

    def test_dirty_workspaces_are_rejected_before_launch(self):
        repository=self.repository()
        (repository/"seed.txt").write_text("pre-existing edit\n")
        result=self.invoke("start","claude","--mode","write","--prompt","Edit",cwd=repository,check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn("clean Git worktree",result.stderr)
        self.assertEqual((repository/"seed.txt").read_text(),"pre-existing edit\n")

    def test_only_one_unresolved_writer_is_allowed_across_checkouts(self):
        first=self.repository("first")
        second=self.repository("second")
        self.create_writer(first)
        result=self.invoke("create","claude","--mode","write","--prompt","Edit",cwd=second,check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn("another write delegation",result.stderr)

    def test_review_and_host_commit_are_required_before_releasing_the_lock(self):
        repository=self.repository()
        session=self.call("start","claude","--mode","write","--prompt","Edit delegated.txt",behavior="write",cwd=repository)["session"]
        question=self.wait_for(session["id"],"question")
        self.wait_for(session["id"],"checkpoint")
        self.assertTrue((repository/"delegated.txt").exists())
        self.call("answer",session["id"],"--reply-to",str(question["seq"]),"--body","Reviewed; no automatic test applies")
        result=self.settled(session["id"])
        self.assertEqual(result["session"]["status"],"completed")
        self.assertIsNone(result["session"]["integrated_at"])
        self.assertEqual(len(self.call("list")["sessions"]),1)
        other=self.repository("other")
        blocked=self.invoke("create","claude","--mode","write","--prompt","Edit",cwd=other,check=False)
        self.assertNotEqual(blocked.returncode,0)
        rejected=self.invoke("integrate",session["id"],check=False)
        self.assertNotEqual(rejected.returncode,0)
        self.commit(repository)
        integrated=self.call("integrate",session["id"])
        self.assertIsNotNone(integrated["session"]["integrated_at"])
        self.assertEqual(self.call("list")["sessions"],[])
        self.assertEqual(self.create_writer(other)["session"]["status"],"created")

    def test_a_write_result_cannot_skip_its_checkpoint_and_review(self):
        repository=self.repository()
        session=self.call("start","claude","--mode","write","--prompt","Edit",behavior="write_without_review",cwd=repository)["session"]
        result=self.settled(session["id"])
        self.assertEqual(result["session"]["status"],"stalled")
        exited=self.wait_for(session["id"],"process_exit")
        self.assertIn("checkpoint",exited["payload"]["stderr_tail"])

    def test_a_writer_with_no_diff_stalls_without_releasing_ownership(self):
        repository=self.repository()
        session=self.call("start","claude","--mode","write","--prompt","Edit","--diff-timeout","0.1",behavior="no_diff",cwd=repository)["session"]
        result=self.settled(session["id"])
        self.assertEqual(result["session"]["status"],"stalled")
        self.assertIn("no workspace diff",result["session"]["error"])
        other=self.repository("other")
        rejected=self.invoke("create","claude","--mode","write","--prompt","Edit",cwd=other,check=False)
        self.assertNotEqual(rejected.returncode,0)
        self.call("cancel",session["id"])
        self.assertEqual(self.create_writer(other)["session"]["status"],"created")
