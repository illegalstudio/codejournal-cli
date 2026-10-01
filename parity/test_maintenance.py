"""Rules, topic maintenance, path movement and startup summary semantics."""
from support import ParityCase


class MaintenanceParityTest(ParityCase):
    def test_rules_topics_and_refs(self):
        self.paired("rules", "append", "Keep commands additive.")
        for engine in self.envs:
            rules = self.invoke(engine, "rules")["rules"]
            self.assertEqual(rules, "Use synthetic data.\nKeep commands additive.")
        entries = self.paired("add", "--kind", "howto", "--title", "Build steps", "--body", "Build synthetic app",
                              "--topics", "builds,build", "--ref", "path:src/app.py")
        self.paired("topics", "merge", "builds", "--into", "build")
        self.paired("refs", "move", "src", "app")
        for engine, result in entries.items():
            shown = self.invoke(engine, "show", result["entry"]["id"], "--no-track")["entry"]
            self.assertEqual(shown["topics"], ["build"])
            self.assertEqual(shown["refs"], [{"kind": "path", "value": "app/app.py"}])
            self.assertEqual(self.invoke(engine, "topics")["topics"], [{"name": "build", "count": 1}])
        self.titles(self.paired("search", "--path", "app"), "entries", ["Build steps"])
        self.titles(self.paired("search", "--path", "src"), "entries", [])
        self.paired("rules", "clear")
        for engine in self.envs:
            self.assertFalse(self.invoke(engine, "rules")["rules"])

    def test_brief_counts_open_questions_and_compact_rules(self):
        self.add_entry()
        self.paired("add", "--kind", "question", "--title", "Open cache question", "--body", "Pending decision")
        self.paired("plan", "create", "--title", "Cache plan", "--body", "Synthetic plan")
        self.paired("doc", "create", "--title", "Cache doc", "--body", "Synthetic doc")
        self.paired("task", "add", "--title", "Cache task", "--body", "Synthetic task")
        for engine in self.envs:
            brief = self.invoke(engine, "brief")
            self.assertEqual(brief.get("rules", brief["project"].get("rules")), "Use synthetic data.")
            self.assertEqual(brief["counts"]["active"], 2)
            self.assertEqual([item["title"] for item in brief["questions"]], ["Open cache question"])
            self.assertEqual([item["title"] for item in brief["plans"]], ["Cache plan"])
            self.assertEqual([item["title"] for item in brief["docs"]], ["Cache doc"])
            compact = self.invoke(engine, "brief", "--compact", json_mode=False).stdout
            self.assertIn("Use synthetic data.", compact)
            self.assertIn("Cache task", compact)

    def test_garden_and_digest_are_actionable(self):
        self.add_entry()
        self.paired("log", "add", "--title", "Synthetic work", "--body", "Completed", "--no-auto-commits")
        self.paired("task", "add", "--title", "Pending work", "--body", "Synthetic task")
        for engine in self.envs:
            garden = self.invoke(engine, "garden", "--dry-run")
            self.assertTrue(garden["dry_run"])
            self.assertEqual(garden["applied"], [])
            self.invoke(engine, "garden")
            digest = self.invoke(engine, "digest", "--since", "today", "--project-only", json_mode=False).stdout
            self.assertIn("Synthetic work", digest)
