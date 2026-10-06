import json
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SKILL = (ROOT / "skill/SKILL.md").read_text()


class OpenCodeCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target/debug/cj")

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="cj-opencode-")
        self.addCleanup(self.temporary.cleanup)
        self.home = pathlib.Path(self.temporary.name)
        self.root = self.home / ".config/opencode"
        self.env = dict(os.environ, HOME=str(self.home), XDG_CONFIG_HOME=str(self.home / ".config"),
                        XDG_STATE_HOME=str(self.home / "state"), XDG_CACHE_HOME=str(self.home / "cache"),
                        OPENCODE_CONFIG_DIR=str(self.root), CODEX_HOME=str(self.home / ".codex"),
                        CLAUDE_CONFIG_DIR=str(self.home / ".claude"))
        for key in ["OPENCODE_DISABLE_EXTERNAL_SKILLS", "OPENCODE_DISABLE_CLAUDE_CODE",
                    "OPENCODE_DISABLE_CLAUDE_CODE_SKILLS"]:
            self.env.pop(key, None)

    def setup(self, *args, check=True):
        result = subprocess.run([self.binary, "setup", "agents", "--agent", "opencode", *args],
                                cwd=self.home, env=self.env, capture_output=True, text=True, check=check)
        return json.loads(result.stdout)["agents"][0] if result.returncode == 0 else result

    def write(self, relative, body=SKILL):
        path = self.home / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body)
        return path

    def hooks(self, action, *args):
        result = subprocess.run([self.binary, "--json", "hooks", action, "--agent", "opencode", *args],
                                cwd=self.home, env=self.env, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
