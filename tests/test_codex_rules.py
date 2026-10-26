import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]


class CodexRulesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target" / "debug" / "cj")

    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="cj-rules-")
        self.addCleanup(temp.cleanup)
        self.codex_home = pathlib.Path(temp.name) / "codex"
        self.codex_home.mkdir()
        self.rules = self.codex_home / "rules" / "code-journal.rules"
        self.env = dict(os.environ, CODEX_HOME=str(self.codex_home))

    def setup_agents(self, *args):
        result = subprocess.run([self.binary, "setup", "agents", "--agent", "codex", *args],
                                env=self.env, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)["agents"][0]

    def test_install_status_and_uninstall_manage_only_the_cj_rules_file(self):
        user_rules = self.codex_home / "rules" / "default.rules"
        user_rules.parent.mkdir()
        user_rules.write_text('prefix_rule(pattern=["ls"], decision="allow")\n')
        self.assertTrue(self.setup_agents("--dry-run")["rules_changed"])
        self.assertFalse(self.rules.exists())
        self.assertTrue(self.setup_agents()["rules_changed"])
        self.assertTrue(self.rules.read_text().startswith("# Managed by cj setup agents"))
        self.assertFalse(self.setup_agents()["rules_changed"])
        self.assertTrue(self.setup_agents("--status")["rules_installed"])
        self.assertTrue(self.setup_agents("--uninstall")["rules_changed"])
        self.assertFalse(self.rules.exists())
        self.assertTrue(user_rules.exists())

    def test_uninstall_keeps_an_unmanaged_file_with_the_same_name(self):
        self.rules.parent.mkdir()
        self.rules.write_text('prefix_rule(pattern=["cj"], decision="allow")\n')
        self.assertFalse(self.setup_agents("--uninstall")["rules_changed"])
        self.assertTrue(self.rules.exists())
        self.assertFalse(self.setup_agents("--status")["rules_installed"])

    @unittest.skipUnless(shutil.which("codex"), "codex CLI not installed")
    def test_codex_allows_journal_commands_and_sandboxes_the_rest(self):
        self.setup_agents()
        def decision(*command):
            result = subprocess.run(["codex", "execpolicy", "check", "--rules", str(self.rules),
                                     *command], capture_output=True, text=True, check=True)
            return json.loads(result.stdout).get("decision")
        self.assertEqual(decision("cj", "brief"), "allow")
        self.assertEqual(decision("cj", "add", "--body", "fact", "--kind", "gotcha", "--title", "x"), "allow")
        for command in [("notifications", "config", "--ntfy", "https://example.test"),
                        ("notify", "--body-file", "/private/file"),
                        ("add", "--kind", "gotcha", "--body-file", "/private/file"),
                        ("task", "note", "id", "--body-file", "/private/file"),
                        ("rules", "set", "--body-file", "/private/file")]:
            self.assertIsNone(decision("cj", *command))
        conflict = subprocess.run([self.binary, "add", "--body", "fact", "--body-file", "/private/file",
                                   "--kind", "gotcha", "--title", "x"], env=self.env, capture_output=True, text=True)
        self.assertNotEqual(conflict.returncode, 0)
        self.assertIn("cannot be used with", conflict.stderr)
        self.assertIsNone(decision("cj", "watch", "start", "--", "ls"))
        self.assertIsNone(decision("cj", "setup", "agents"))
        self.assertIsNone(decision("cj", "--server", "https://example.test", "add"))


if __name__ == "__main__":
    unittest.main()
