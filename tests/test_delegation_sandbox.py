import json
import os
import subprocess
from pathlib import Path
from delegation_support import DelegationCase


PROBE = r'''
if os.environ.get("CJ_FAKE_BEHAVIOR") == "readonly_probe":
    for path in [Path("seed.txt"), Path("ignored.txt"), Path(".git/config")]:
        try:
            path.write_text("damaged")
        except OSError:
            pass
        else:
            raise RuntimeError("Read-only write was allowed: " + str(path))
    scratch = Path(os.environ["TMPDIR"]) / "probe.txt"
    scratch.write_text("isolated probe")
    scratch.unlink()
    subprocess.run(["/bin/sh", "-c", 'trap \'/bin/rm -rf "$PWD"\' EXIT'], capture_output=True)
    if not Path("seed.txt").exists() or not Path("ignored.txt").exists():
        raise RuntimeError("Destructive cleanup succeeded")
'''


class DelegationSandboxTest(DelegationCase):
    def test_shell_descendants_cannot_write_or_delete_the_checkout(self):
        repo = self.repository()
        ignored = repo / "ignored.txt"
        ignored.write_text("preserved ignored data")
        (repo / ".gitignore").write_text("ignored.txt\n")
        self.commit(repo)
        for provider in ("claude", "codex", "agent", "grok", "kimi", "pi"):
            fake = self.bin / provider
            fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")', PROBE + '\npublish("ready", "Protocol loaded")'))
        before = self.git(repo, "rev-parse", "HEAD")
        for target in ("claude", "codex", "cursor", "grok", "kimi", "pi"):
            with self.subTest(target=target):
                identifier = self.start("readonly_probe", target=target, cwd=repo)
                snapshot = self.settled(identifier)
                self.assertEqual(snapshot["session"]["status"], "completed", json.dumps(snapshot))
                self.assertEqual((repo / "seed.txt").read_text(), "baseline\n")
                self.assertEqual(ignored.read_text(), "preserved ignored data")
                self.assertEqual(self.git(repo, "rev-parse", "HEAD"), before)
                exits = [event for event in snapshot["events"] if event["kind"] == "process_exit"]
                self.assertNotIn("external sender is unknown", exits[-1]["payload"]["reason"])
        self.assertFalse(list((self.root / "state/codejournal/delegations").glob("sandbox-*")))

    def test_synchronous_run_has_the_same_filesystem_boundary(self):
        repo = self.repository()
        fake = self.bin / "claude"
        fake.write_text(f"#!{os.sys.executable}\nfrom pathlib import Path\ntry:\n Path('seed.txt').write_text('damage')\nexcept OSError:\n print('write rejected')\nelse:\n raise RuntimeError('unprotected')\n")
        result = self.invoke("run", "claude", "--prompt", "Inspect", cwd=repo)
        self.assertIn("write rejected", result.stdout)
        self.assertEqual((repo / "seed.txt").read_text(), "baseline\n")

    def test_provider_state_may_not_overlap_the_repository(self):
        repo = self.repository()
        for target, variable in [("codex", "CODEX_HOME"), ("grok", "GROK_HOME"),
                                 ("kimi", "KIMI_CODE_HOME"), ("pi", "PI_CODING_AGENT_DIR")]:
            with self.subTest(target=target):
                env = self.env.copy()
                env[variable] = str(repo / "provider-state")
                result = self.invoke("run", target, "--prompt", "Inspect", cwd=repo, env=env, check=False)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("overlaps", result.stderr)
                self.assertFalse((repo / "provider-state").exists())

    def test_failed_sandbox_preparation_clears_the_supervisor(self):
        repo = self.repository()
        env = self.env.copy()
        env["CODEX_HOME"] = str(repo)
        ident = self.start(target="codex", cwd=repo, env=env)
        snapshot = self.settled(ident)
        self.assertEqual(snapshot["session"]["status"], "stalled")
        self.assertIn("overlaps", snapshot["session"]["error"])
