import json
import pathlib
import subprocess
import unittest
from opencode_support import OpenCodeCase, SKILL


class OpenCodeSetupTest(OpenCodeCase):
    def test_native_install_preview_status_refresh_and_uninstall(self):
        self.assertTrue(self.setup("--dry-run")["changed"])
        self.assertFalse(self.root.exists())
        self.assertFalse(self.setup("--status")["skill_installed"])
        installed = self.setup()
        skill = pathlib.Path(installed["skill"])
        self.assertEqual(skill.read_text(), SKILL)
        self.assertIn("SessionStart", installed["hooks_installed"])
        self.assertFalse(self.setup()["changed"])
        self.assertTrue(self.setup("--status")["skill_current"])
        self.assertFalse(self.setup("--refresh")["changed"])
        self.setup("--uninstall")
        self.assertFalse(skill.exists())
        self.assertFalse((self.root / "plugins/code-journal.js").exists())

    def test_reuses_claude_skill_and_never_removes_it(self):
        shared = self.write(".claude/skills/code-journal/SKILL.md")
        before = shared.stat().st_mtime_ns
        installed = self.setup()
        self.assertEqual(installed["skill"], str(shared))
        self.assertTrue(installed["skill_reused"])
        self.assertFalse((self.root / "skills/code-journal/SKILL.md").exists())
        self.setup("--refresh")
        self.assertEqual(shared.stat().st_mtime_ns, before)
        self.setup("--uninstall")
        self.assertEqual(shared.read_text(), SKILL)

    def test_conflicting_copies_are_reported_before_any_writes(self):
        self.write(".claude/skills/code-journal/SKILL.md")
        self.write(".agents/skills/code-journal/SKILL.md", SKILL + "\nCustomized\n")
        failed = self.setup(check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("Conflicting", failed.stderr)
        self.assertFalse(self.root.exists())
        self.assertTrue(self.setup("--status")["skill_conflict"])

    def test_ignores_claude_override_that_opencode_does_not_discover(self):
        self.env["CLAUDE_CONFIG_DIR"] = str(self.home / "claude-custom")
        self.write("claude-custom/skills/code-journal/SKILL.md")
        self.assertFalse(self.setup()["skill_reused"])

    def test_custom_config_root_and_external_discovery_disabled(self):
        self.root = self.home / "custom opencode"
        self.env["OPENCODE_CONFIG_DIR"] = str(self.root)
        self.env["OPENCODE_DISABLE_EXTERNAL_SKILLS"] = "true"
        self.write(".claude/skills/code-journal/SKILL.md")
        self.assertEqual(self.setup()["skill"], str(self.root / "skills/code-journal/SKILL.md"))

    def test_refresh_skips_unconfigured_opencode(self):
        self.root.mkdir(parents=True)
        result = subprocess.run([self.binary, "setup", "agents", "--refresh"], env=self.env,
                                capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout)["agents"], [])
        self.assertEqual(list(self.root.iterdir()), [])

    def test_uses_xdg_config_home_without_custom_root(self):
        self.env.pop("OPENCODE_CONFIG_DIR")
        self.assertEqual(self.setup()["skill"], str(self.root / "skills/code-journal/SKILL.md"))

    def test_shared_copy_can_replace_owned_native_copy(self):
        native = pathlib.Path(self.setup()["skill"])
        shared = self.write(".agents/skills/code-journal/SKILL.md")
        self.assertEqual(self.setup()["skill"], str(shared))
        self.assertFalse(native.exists())
        self.assertEqual(shared.read_text(), SKILL)


if __name__ == "__main__":
    unittest.main()
