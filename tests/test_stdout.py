import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
BODY = "Synthetic journal text.\n" * 20000


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        item = {"id": "a" * 32, "body": BODY, "title": "Synthetic", "revision": 1}
        result = {"doc": item, "log": item, "project": {"slug": "test"},
                  "records": [{"type": "entry", "body": BODY}]}
        data = json.dumps(result).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


class StdoutTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = str(ROOT / "target/debug/cj")
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.worker = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.worker.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.worker.join(timeout=2)

    def setUp(self):
        folder = tempfile.TemporaryDirectory(prefix="cj-stdout-")
        self.addCleanup(folder.cleanup)
        base = pathlib.Path(folder.name)
        self.base = base
        config = base / "config/codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"}))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(base / "config"),
                        XDG_STATE_HOME=str(base / "state"), XDG_CACHE_HOME=str(base / "cache"),
                        CJ_TOKEN="test-token", CODE_JOURNAL_HOOK_FLUSH="off")

    def command(self, *args):
        return [self.binary, "--project", "test", *args]

    def test_closed_and_partially_read_pipes_exit_without_errors(self):
        commands = [("--json", "doc", "show", "aaaaaaaa", "--current-only"),
                    ("doc", "show", "aaaaaaaa"), ("doc", "show", "aaaaaaaa", "--body"),
                    ("log", "show", "aaaaaaaa", "--body"), ("export", "--all-projects")]
        for command in commands:
            for prefix in [0, 128]:
                with self.subTest(command=command, prefix=prefix):
                    with subprocess.Popen(self.command(*command), env=self.env, cwd=ROOT,
                                          stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
                        if prefix:
                            self.assertTrue(process.stdout.read(prefix))
                        process.stdout.close()
                        process.stdout = None
                        _, error = process.communicate(timeout=20)
                    self.assertEqual(process.returncode, 0, error.decode())
                    self.assertEqual(error, b"")

    @unittest.skipUnless(pathlib.Path("/dev/full").exists(), "Requires a failing output device")
    def test_other_stdout_errors_still_fail(self):
        with open("/dev/full", "wb") as output:
            result = subprocess.run(self.command("--json", "doc", "show", "aaaaaaaa"),
                                    env=self.env, cwd=ROOT, stdout=output, stderr=subprocess.PIPE, timeout=20)
        self.assertEqual(result.returncode, 1)
        self.assertIn(b"stdout", result.stderr)
        self.assertNotIn(b"panicked", result.stderr)

    def test_unrelated_file_errors_still_fail(self):
        result = subprocess.run(self.command("--json", "export", "--output", str(self.base / "missing/journal.jsonl")),
                                env=self.env, cwd=ROOT, capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 1)
        self.assertNotIn(b"panicked", result.stderr)
