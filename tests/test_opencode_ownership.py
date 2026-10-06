import json
import pathlib
import unittest
from opencode_support import OpenCodeCase, SKILL


class OpenCodeOwnershipTest(OpenCodeCase):
    def test_customized_plugin_is_preserved_with_its_helpers(self):
        self.setup()
        plugin = self.root / "plugins/code-journal.js"
        original = plugin.read_text() + "\n// My customization\n"
        plugin.write_text(original)
        failed = self.setup(check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("customized", failed.stderr)
        removed = self.setup("--uninstall")
        self.assertEqual(plugin.read_text(), original)
        self.assertTrue((self.root / "plugins/code-journal/plugin.mjs").is_file())
        self.assertEqual(removed["plugin"]["modified"], [str(plugin)])
        self.assertEqual(removed["plugin"]["removed"], [])

    def test_unrelated_config_plugins_and_instructions_are_unchanged(self):
        config = self.write(".config/opencode/opencode.jsonc", '// User config\n{"plugin": []}\n')
        instructions = self.write(".config/opencode/AGENTS.md", "Keep user instructions.\n")
        plugin = self.write(".config/opencode/plugins/other.js", "export const Other = async () => ({});\n")
        original = {path: path.read_bytes() for path in [config, instructions, plugin]}
        self.setup()
        self.setup("--uninstall")
        for path, body in original.items():
            self.assertEqual(path.read_bytes(), body)

    def test_refresh_updates_owned_old_skill_with_backup(self):
        self.setup()
        skill = self.root / "skills/code-journal/SKILL.md"
        skill.write_text("older managed copy")
        import hashlib
        manifest = self.root / ".code-journal-skill.json"
        manifest.write_text(json.dumps({"skills/code-journal/SKILL.md": hashlib.sha256(skill.read_bytes()).hexdigest()}))
        self.setup("--refresh")
        self.assertEqual(skill.read_text(), SKILL)
        self.assertEqual(len(list(skill.parent.glob("*.cj-backup-*.md"))), 1)

    def test_customized_native_skill_blocks_install_and_survives_uninstall(self):
        skill = pathlib.Path(self.setup()["skill"])
        original = SKILL + "\nMy instructions\n"
        skill.write_text(original)
        self.assertNotEqual(self.setup(check=False).returncode, 0)
        self.setup("--uninstall")
        self.assertEqual(skill.read_text(), original)

    def test_hooks_commands_manage_only_the_plugin(self):
        self.assertTrue(self.hooks("install", "--dry-run")["agents"][0]["changed"])
        self.assertFalse(self.root.exists())
        self.hooks("install")
        status = self.hooks("status")["agents"][0]
        self.assertIn("Stop", status["installed"])
        self.assertEqual(status["sessions_seen"], 0)
        self.assertIsNone(status["last_event_at"])
        self.assertFalse((self.root / "skills").exists())
        self.hooks("uninstall")
        self.assertEqual(self.hooks("status")["agents"][0]["installed"], [])

    def test_identical_existing_copies_are_reported_but_not_rewritten(self):
        self.write(".claude/skills/code-journal/SKILL.md")
        self.write(".agents/skills/code-journal/SKILL.md")
        installed = self.setup()
        self.assertEqual(len(installed["skill_candidates"]), 2)
        self.assertFalse((self.root / "skills/code-journal/SKILL.md").exists())

    def test_invalid_existing_skill_is_not_claimed_or_replaced(self):
        skill = self.write(".config/opencode/skills/code-journal/SKILL.md", "Private notes")
        status = self.setup("--status")
        self.assertTrue(status["skill_invalid"])
        self.assertFalse(status["skill_installed"])
        self.assertNotEqual(self.setup(check=False).returncode, 0)
        self.assertEqual(skill.read_text(), "Private notes")
        self.assertFalse((self.root / "plugins").exists())

    def test_windows_line_endings_and_quoted_names_are_discovered(self):
        shared = self.write(".claude/skills/code-journal/SKILL.md", SKILL.replace("name: code-journal", "name: 'code-journal'").replace("\n", "\r\n"))
        self.assertEqual(self.setup()["skill"], str(shared))

    def test_second_plugin_location_is_reported_before_installing(self):
        other = self.write(".config/opencode/plugin/code-journal.ts", "// User plugin\n")
        self.assertNotEqual(self.setup(check=False).returncode, 0)
        self.assertFalse((self.root / "skills/code-journal/SKILL.md").exists())
        self.assertEqual(self.setup("--status")["plugin"]["conflicts"], [str(other)])
        self.assertEqual(other.read_text(), "// User plugin\n")

    def test_config_directory_alias_does_not_retire_the_only_skill(self):
        self.setup()
        alias = self.home / "opencode-alias"
        alias.symlink_to(self.root, target_is_directory=True)
        self.env["OPENCODE_CONFIG_DIR"] = str(alias)
        refreshed = self.setup("--refresh")
        self.assertFalse(refreshed["skill_reused"])
        self.assertTrue((self.root / "skills/code-journal/SKILL.md").is_file())
        self.assertEqual(refreshed["plugin"]["conflicts"], [])


if __name__ == "__main__":
    unittest.main()
