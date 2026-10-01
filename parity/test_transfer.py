"""JSONL compatibility, idempotence and historical record preservation."""
import json
from support import ParityCase


class TransferParityTest(ParityCase):
    def test_python_export_imports_into_hosted_and_roundtrips(self):
        self.invoke("python", "add", "--kind", "gotcha", "--title", "Imported cache", "--body", "Historical synthetic fact", "--topics", "cache")
        self.invoke("python", "plan", "create", "--title", "Imported plan", "--body", "- [ ] Validate")
        self.invoke("python", "doc", "create", "--title", "Imported doc", "--body", "Historical docs", "--ref", "path:README.md")
        self.invoke("python", "task", "add", "--title", "Imported task", "--body", "Historical task")
        self.invoke("python", "log", "add", "--title", "Imported work", "--body", "Historical log", "--no-auto-commits")
        legacy = self.invoke("python", "export", json_mode=False).stdout
        path = self.base / "python.jsonl"
        path.write_text(legacy)
        self.invoke("rust", "import", path)
        for command, key, title in [("search", "entries", "Imported cache"), ("plan", "plans", "Imported plan"),
                                    ("doc", "docs", "Imported doc"), ("task", "tasks", "Imported task"), ("log", "logs", "Imported work")]:
            args = (command,) if command == "search" else (command, "list")
            self.titles(self.paired(*args), key, [title])
        self.invoke("rust", "import", path)
        self.titles(self.paired("search"), "entries", ["Imported cache"])
        hosted = self.invoke("rust", "export", json_mode=False).stdout
        path.write_text(hosted)
        self.envs["python"]["CODE_JOURNAL_DB_PATH"] = str(self.base / "fresh-python.sqlite")
        self.invoke("python", "import", path)
        self.titles(self.paired("search"), "entries", ["Imported cache"])
        records = [json.loads(line) for line in hosted.splitlines()]
        self.assertEqual({record["type"] for record in records}, {"project", "entry", "plan", "task", "log"})

    def test_export_file_and_repeated_import(self):
        self.add_entry()
        for engine in self.envs:
            path = self.base / f"{engine}.jsonl"
            self.invoke(engine, "export", "--output", path, json_mode=False)
            records = [json.loads(line) for line in path.read_text().splitlines()]
            self.assertEqual([record["title"] for record in records if record["type"] == "entry"], ["Cache invalidation"])
            self.invoke(engine, "import", path)
        self.titles(self.paired("search"), "entries", ["Cache invalidation"])
