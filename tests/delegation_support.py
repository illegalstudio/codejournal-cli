from contextlib import closing
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from delegation_fake import FAKE

ROOT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(os.name == "posix", "fake provider executables use POSIX shebangs")
class DelegationCase(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="cj-standalone-delegation-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.client = self.bin / "cj"
        binary = Path(os.environ.get("CJ_DELEGATION_TEST_BINARY", ROOT / "target/debug/cj"))
        shutil.copy2(binary, self.client)
        for target in ("claude", "codex", "agent", "cursor-agent", "grok", "kimi", "pi"):
            fake = self.bin / target
            fake.write_text(f"#!{sys.executable}\n" + FAKE)
            fake.chmod(0o755)
        self.env = os.environ.copy()
        for key in list(self.env):
            if key.startswith(("CJ_", "CODE_JOURNAL_", "ME_DELEGATION_")):
                del self.env[key]
        self.env.update({"HOME":str(self.root), "XDG_CONFIG_HOME":str(self.root/"config"),
            "XDG_STATE_HOME":str(self.root/"state"), "XDG_CACHE_HOME":str(self.root/"cache"),
            "XDG_DATA_HOME":str(self.root/"data"), "XDG_RUNTIME_DIR":str(self.root/"runtime-a"),
            "PATH":str(self.bin), "CJ_DELEGATION_JOURNAL":"off", "CODE_JOURNAL_AUTO_SYNC":"off",
            "CODE_JOURNAL_HOOK_FLUSH":"off", "CODEX_HOME":str(self.root/"codex")})
        self.env.update({"CODEX_THREAD_ID":"synthetic-delegation-host","CODE_JOURNAL_AGENT":"codex"})
        self.addCleanup(self.stop_sessions)

    def invoke(self, *args, behavior=None, env=None, cwd=None, check=True):
        environment = (env or self.env).copy()
        if behavior:
            environment["CJ_FAKE_BEHAVIOR"] = behavior
        result = subprocess.run([str(self.client),"delegation",*args], env=environment,
            cwd=cwd or self.root, text=True, capture_output=True, timeout=15)
        if check and result.returncode:
            self.fail(result.stderr)
        return result

    def call(self, *args, **kwargs):
        return json.loads(self.invoke(*args, **kwargs).stdout)

    def start(self, behavior="finish", target="claude", **kwargs):
        return self.call("start",target,"--prompt","Bounded synthetic task","--stall-timeout","3",
            behavior=behavior, **kwargs)["session"]["id"]

    def wait_for(self, delegation_id, kind):
        deadline = time.monotonic()+10
        while time.monotonic() < deadline:
            snapshot = self.call("status",delegation_id,"--full")
            for event in snapshot["events"]:
                if event["kind"] == kind:
                    return event
            time.sleep(.04)
        self.fail("Missing event: "+kind)

    def settled(self, delegation_id):
        deadline = time.monotonic()+10
        while time.monotonic() < deadline:
            snapshot = self.call("status",delegation_id,"--full")
            if snapshot["session"]["status"] in ("completed","failed","cancelled","stalled") and not snapshot["session"]["supervisor_pid"]:
                return snapshot
            time.sleep(.04)
        self.fail("Supervisor did not finish")

    def stop_sessions(self):
        database = self.root/"state/codejournal/delegations/delegations.sqlite3"
        if not database.exists():
            return
        sessions = self.call("list","--all")["sessions"]
        for session in sessions:
            if session["status"] not in ("completed","failed","cancelled"):
                self.invoke("cancel",session["id"],check=False)
        deadline = time.monotonic()+5
        while time.monotonic()<deadline:
            with closing(sqlite3.connect(database)) as connection, connection:
                active = connection.execute("SELECT count(*) FROM delegations WHERE supervisor_pid IS NOT NULL").fetchone()[0]
            if not active:
                return
            time.sleep(.04)

    def repository(self, name="repo"):
        git = shutil.which("git")
        self.assertIsNotNone(git)
        link = self.bin/"git"
        if not link.exists():
            link.symlink_to(git)
        path = self.root/name
        path.mkdir()
        self.git(path,"init","--quiet")
        (path/"seed.txt").write_text("baseline\n")
        self.commit(path)
        return path

    def git(self,path,*args):
        return subprocess.run([str(self.bin/"git"),"-C",str(path),*args],env=self.env,
            check=True,text=True,capture_output=True).stdout

    def commit(self,path):
        self.git(path,"add",".")
        self.git(path,"-c","user.name=Delegation test","-c","user.email=test@example.invalid",
            "-c","commit.gpgsign=false","commit","--quiet","-m","test: synthetic baseline")
