from contextlib import closing
import sqlite3
import subprocess
import sys
import time
from delegation_support import DelegationCase


class DelegationGitContentionTest(DelegationCase):
    def test_slow_checkpoint_git_does_not_lock_other_delegations(self):
        self.check_git_lock(False)

    def test_slow_integration_git_does_not_lock_other_delegations(self):
        self.check_git_lock(True)

    def check_git_lock(self, integration):
        repository = self.repository()
        session = self.call("create", "claude", "--mode", "write", "--prompt", "Inspect the diff", cwd=repository)["session"]
        (repository/"seed.txt").write_text("delegated edit\n")
        if integration:
            self.commit(repository)
        database = self.root/"state/codejournal/delegations/delegations.sqlite3"
        with closing(sqlite3.connect(database)) as db, db:
            db.execute("UPDATE delegations SET status=?,attempt_token=? WHERE id=?", ("completed" if integration else "running", "synthetic-attempt", session["id"]))
        git = self.bin/"git"
        executable = git.resolve()
        git.unlink()
        entered = self.root/"git-entered"
        release = self.root/"release-git"
        git.write_text(f'''#!{sys.executable}
import os, sys, time
from pathlib import Path
if "status" in sys.argv[1:]:
    Path({str(entered)!r}).touch()
    while not Path({str(release)!r}).exists(): time.sleep(.02)
os.execv({str(executable)!r}, [{str(executable)!r}, *sys.argv[1:]])
''')
        git.chmod(0o755)
        environment = self.env.copy()
        if integration:
            arguments = ["integrate", session["id"]]
        else:
            environment.update({"CJ_DELEGATION_ID": session["id"], "CJ_DELEGATION_TOKEN": "synthetic-attempt", "CJ_DELEGATION_ROLE": "guest"})
            arguments = ["publish", session["id"], "--role", "guest", "--kind", "checkpoint", "--body", "Verified diff"]
        child = subprocess.Popen([str(self.client), "delegation", *arguments], env=environment, cwd=repository, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        locked = False
        try:
            deadline = time.monotonic()+5
            while not entered.exists() and time.monotonic() < deadline:
                time.sleep(.02)
            self.assertTrue(entered.exists(), "Git validation did not start")
            with closing(sqlite3.connect(database, timeout=.3)) as other:
                try:
                    other.execute("BEGIN IMMEDIATE")
                    other.rollback()
                except sqlite3.OperationalError as error:
                    if "locked" not in str(error):
                        raise
                    locked = True
        finally:
            release.touch()
            try:
                _stdout, stderr = child.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.communicate()
                raise
        self.assertEqual(child.returncode, 0, stderr)
        self.assertFalse(locked, "Slow Git held the shared delegation write lock")
