import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
PROJECT = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
ENTRY = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
PLAN = "cccccccccccccccccccccccccccccccc"
LOG = "dddddddddddddddddddddddddddddddd"
RECORDS = [
    {"type": "project", "id": PROJECT, "slug": "fixture", "name": "Fixture"},
    {"type": "entry", "id": ENTRY, "project_id": PROJECT, "kind": "gotcha",
     "title": "Portable", "body": "Test", "topics": [], "refs": []},
    {"type": "log", "id": LOG, "project_id": PROJECT, "plan_id": PLAN,
     "title": "Work", "body": "Done"},
    {"type": "plan", "id": PLAN, "project_id": PROJECT,
     "title": "Plan", "body": "Steps", "revisions": []},
]


class Handler(http.server.BaseHTTPRequestHandler):
    batches = []
    finalizations = []

    def log_message(self, *_args):
        pass

    def respond(self, payload):
        data = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path.endswith("/export?project=fixture"):
            self.respond({"records": RECORDS})
        else:
            self.respond({"error": "missing"})

    def do_PUT(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.batches.append((self.path, body["records"]))
        self.respond({"stored": True})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.finalizations.append((self.path, body))
        self.respond({"counts": {"projects": 1, "entries": 1, "plans": 1,
            "logs": 1, "notifications": 0, "tasks": 0, "feedback": 0, "skipped": 0},
            "project_map": {"fixture": "fixture"}})


class TransferTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target" / "debug" / "cj")
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.worker = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.worker.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.worker.join(timeout=2)

    def setUp(self):
        Handler.batches.clear()
        Handler.finalizations.clear()
        self.temp = tempfile.TemporaryDirectory(prefix="cj-transfer-")
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        config = self.base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(self.base / "config"),
                        XDG_STATE_HOME=str(self.base / "state"), CJ_TOKEN="test-token")

    def cli(self, *args):
        return subprocess.run([self.binary, "--project", "fixture", *args],
            cwd=self.base, env=self.env, capture_output=True, text=True, timeout=10,
            check=True).stdout

    def test_export_is_json_lines_and_import_resolves_old_hex_ids(self):
        exported = self.cli("export")
        self.assertEqual(len(exported.splitlines()), 4)
        self.assertEqual(json.loads(exported.splitlines()[0])["slug"], "fixture")
        source = self.base / "records.jsonl"
        source.write_text(exported)
        self.assertIn("Imported 1 entries", self.cli("import", str(source)))
        batch = Handler.batches[0][1]
        self.assertTrue(Handler.batches[0][0].endswith("/chunks/0"))
        self.assertEqual(Handler.finalizations[0][1], {"chunks": 1})
        self.assertEqual([row["type"] for row in batch],
                         ["project", "entry", "plan", "log"])
        self.assertEqual(batch[1]["id"], "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")
        self.assertEqual(batch[3]["plan_id"], "cccccccc-cccc-cccc-cccc-cccccccccccc")
        self.assertEqual(batch[1]["project_slug"], "fixture")

    def test_large_unicode_records_split_into_bounded_chunks(self):
        source = self.base / "bounded.jsonl"
        records = [dict(RECORDS[0]), *[dict(RECORDS[1], body="😀" * 80_000) for _ in range(3)]]
        source.write_text("\n".join(json.dumps(record) for record in records))
        self.cli("import", str(source))
        self.assertEqual(len(Handler.batches), 3)
        self.assertEqual(Handler.finalizations[0][1], {"chunks": 3})
        for _, chunk in Handler.batches:
            self.assertLessEqual(len(json.dumps(chunk, ensure_ascii=False).encode()), 524_288)
        Handler.batches.clear()
        Handler.finalizations.clear()
        source.write_text(json.dumps(dict(RECORDS[0], rules="x" * 524_288)))
        result = subprocess.run([self.binary, "import", str(source)], env=self.env, cwd=self.base,
                                capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("chunk budget", result.stderr)
        self.assertFalse(Handler.batches)
        self.assertFalse(Handler.finalizations)
