"""Shared knowledge behavior, including write validation and read tracking."""
from support import ParityCase


class KnowledgeParityTest(ParityCase):
    def test_search_filters_prefix_and_any(self):
        self.add_entry()
        self.paired("add", "--kind", "decision", "--title", "Database indexes", "--body", "Query planner", "--topics", "database")
        for args, expected in [(("cache",), ["Cache invalidation"]),
                               (("cache planner",), []),
                               (("cache planner", "--any"), ["Cache invalidation", "Database indexes"]),
                               (("invalid", "--prefix"), ["Cache invalidation"]),
                               (("--topic", "database"), ["Database indexes"]),
                               (("--kind", "gotcha"), ["Cache invalidation"])]:
            with self.subTest(args=args):
                self.titles(self.paired("search", *args), "entries", expected)
        self.titles(self.paired("recent", "--kind", "decision"), "entries", ["Database indexes"])
        for result in self.paired("search", "cache").values():
            self.assertEqual(result["project"]["provides"], {"auto": [], "manual": []})

    def test_usage_prefix_resolution_and_metadata(self):
        entries = self.add_entry()
        for engine, result in entries.items():
            identity = result["entry"]["id"].replace("-", "")[:8]
            self.invoke(engine, "show", identity)
            self.invoke(engine, "show", identity, "--no-track")
            self.invoke(engine, "entry", "flag", identity, "--wrong", "--note", "Outdated advice")
            self.invoke(engine, "entry", "flag", identity, "--helpful")
            self.invoke(engine, "entry", "scope", identity, "global")
            self.invoke(engine, "entry", "set-agent", identity, "claude")
            item = self.invoke(engine, "show", identity, "--no-track")["entry"]
            self.assertEqual((item["scope"], item["agent"]), ("global", "claude"))
            self.assertEqual({key: item["usage"][key] for key in ["views", "helpful", "wrong"]},
                             {"views": 1, "helpful": 1, "wrong": 1})
            self.assertEqual(item["topics"], ["cache"])

    def test_duplicates_and_obsolete_status(self):
        entries = self.add_entry()
        for engine, result in entries.items():
            duplicate = self.invoke(engine, "add", "--kind", "gotcha", "--title", "Cache invalidation",
                                    "--body", "Different advice", check=False)
            self.assertNotEqual(duplicate.returncode, 0)
            self.assertIn("title", duplicate.stderr.lower())
            self.invoke(engine, "obsolete", result["entry"]["id"])
        self.titles(self.paired("search", "cache"), "entries", [])
        self.titles(self.paired("search", "cache", "--all-statuses"), "entries", ["Cache invalidation"])

    def test_answer_and_supersede(self):
        questions = self.paired("add", "--kind", "question", "--title", "Which cache?", "--body", "Choose cache", "--topics", "cache")
        for engine, result in questions.items():
            old = result["entry"]["id"]
            answered = self.invoke(engine, "answer", old, "--title", "Use Redis", "--body", "Shared cache")
            self.assertEqual(answered["answer"]["title"], "Use Redis")
            self.assertEqual(self.invoke(engine, "show", old, "--no-track")["entry"]["status"], "superseded")
        self.titles(self.paired("search", "cache"), "entries", ["Use Redis"])

    def test_validation_and_stdin(self):
        for engine in self.envs:
            for args in [("add", "--kind", "invalid", "--title", "Invalid", "--body", "Body"),
                         ("add", "--kind", "gotcha", "--title", " ", "--body", "Body"),
                         ("show", "00000000")]:
                self.assertNotEqual(self.invoke(engine, *args, check=False).returncode, 0)
            item = self.invoke(engine, "add", "--kind", "howto", "--title", "Piped knowledge", input="Read from stdin\n")
            shown = self.invoke(engine, "show", item["entry"]["id"], "--no-track")["entry"]
            self.assertEqual(shown["body"], "Read from stdin")
