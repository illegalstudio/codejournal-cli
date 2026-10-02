import json
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class AgentSetupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target/debug/cj")

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="cj-agent-setup-")
        self.addCleanup(self.temporary.cleanup)
        self.home = pathlib.Path(self.temporary.name)
        self.env = dict(os.environ, HOME=str(self.home), CODEX_HOME=str(self.home / ".codex"),
                        CLAUDE_CONFIG_DIR=str(self.home / ".claude"), KIMI_CODE_HOME=str(self.home / ".kimi-code"),
                        PI_CODING_AGENT_DIR=str(self.home / ".pi/agent"))

    def setup(self, *args, check=True):
        result = subprocess.run([self.binary, "setup", "agents", *args], env=self.env,
                                capture_output=True, text=True, check=check)
        return json.loads(result.stdout) if result.returncode == 0 else result

    def test_additional_agents_preserve_instructions_and_support_uninstall(self):
        for name, root in [("grok", ".grok"), ("kimi", ".kimi-code"), ("pi", ".pi/agent")]:
            with self.subTest(agent=name):
                directory = self.home / root
                directory.mkdir(parents=True)
                instructions = directory / "AGENTS.md"
                original = "Keep my instructions exactly.\n"
                instructions.write_text(original)
                preview = self.setup("--agent", name, "--dry-run")["agents"][0]
                self.assertTrue(preview["instructions_changed"])
                self.assertEqual(instructions.read_text(), original)
                installed = self.setup("--agent", name)["agents"][0]
                skill = pathlib.Path(installed["skill"])
                self.assertTrue(skill.exists())
                self.assertIn("cj brief", instructions.read_text())
                self.assertIn("https://github.com/illegalstudio/codejournal-cli", instructions.read_text())
                self.assertIn("brew install illegalstudio/tap/codejournal-cli", instructions.read_text())
                self.assertIn("brew upgrade illegalstudio/tap/codejournal-cli", skill.read_text())
                self.assertIn("mise use -g github:illegalstudio/codejournal-cli@VERSION", skill.read_text())
                self.assertIn("tell the user", skill.read_text())
                self.assertEqual(installed["hooks_installed"], [])
                self.assertFalse(self.setup("--agent", name)["agents"][0]["instructions_changed"])
                self.setup("--agent", name, "--uninstall")
                self.assertEqual(instructions.read_text(), original)
                self.assertFalse(skill.exists())

    def test_refresh_does_not_enable_unconfigured_agents_and_updates_existing_skill(self):
        (self.home / ".kimi-code").mkdir()
        (self.home / ".codex").mkdir()
        self.assertEqual(self.setup("--refresh")["agents"], [])
        skill = pathlib.Path(self.setup("--agent", "kimi")["agents"][0]["skill"])
        skill.write_text("previous installed skill")
        refreshed = self.setup("--refresh")["agents"]
        self.assertEqual([item["agent"] for item in refreshed], ["kimi"])
        self.assertEqual(skill.read_text(), (ROOT / "skill/SKILL.md").read_text())
        self.assertFalse((self.home / ".codex/hooks.json").exists())
        self.assertEqual(len(list(skill.parent.glob("*.cj-backup-*.md"))), 1)

    def test_broken_instruction_markers_are_not_replaced(self):
        directory = self.home / ".kimi-code"
        directory.mkdir()
        instructions = directory / "AGENTS.md"
        original = "User text\n<!-- code-journal:start -->\nincomplete"
        instructions.write_text(original)
        failed = self.setup("--agent", "kimi", check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertEqual(instructions.read_text(), original)
        self.assertFalse((directory / "skills/code-journal/SKILL.md").exists())


if __name__ == "__main__":
    unittest.main()
