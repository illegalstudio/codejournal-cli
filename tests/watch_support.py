import http.server
import json
import os
import pathlib
import subprocess
import tempfile
import threading
import time
import unittest
from watch_server import Handler

ROOT = pathlib.Path(__file__).resolve().parents[1]


class WatchCase(unittest.TestCase):
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
        Handler.reset()
        self.temp = tempfile.TemporaryDirectory(prefix="cj-watch-")
        self.addCleanup(self.temp.cleanup)
        self.addCleanup(self.stop_workers)
        config = pathlib.Path(self.temp.name) / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": f"http://127.0.0.1:{self.server.server_port}", "tenant": "demo"
        }))
        self.env = dict(os.environ, XDG_CONFIG_HOME=str(config.parent),
                        XDG_STATE_HOME=str(pathlib.Path(self.temp.name) / "state"),
                        XDG_CACHE_HOME=str(pathlib.Path(self.temp.name) / "cache"),
                        XDG_DATA_HOME=str(pathlib.Path(self.temp.name) / "data"), CJ_TOKEN="test-token")

    def cli(self, *args):
        return subprocess.run([self.binary, "--project", "p", *args], env=self.env,
                              cwd=ROOT, capture_output=True, text=True, timeout=15, check=True).stdout

    def wait(self, ident, status):
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline:
            if Handler.watches[ident]["status"] == status:
                return Handler.watches[ident]
            time.sleep(0.05)
        self.fail(f"watch {ident} did not reach {status}")

    def stop_workers(self):
        state = pathlib.Path(self.temp.name) / "state" / "codejournal" / "watches"
        for record in state.glob("*.pid"):
            data = json.loads(record.read_text())
            pid = data["pid"] if isinstance(data, dict) else data
            try:
                os.kill(pid, 15)
            except ProcessLookupError:
                continue
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline and list(state.glob("*.pid")):
            time.sleep(0.05)
