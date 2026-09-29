import json
import os
import pathlib
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]


class TokenAccessTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True, capture_output=True)
        cls.binary = str(ROOT / "target" / "debug" / "cj")

    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="cj-token-")
        self.addCleanup(temp.cleanup)
        base = pathlib.Path(temp.name)
        config = base / "config" / "codejournal"
        config.mkdir(parents=True)
        (config / "config.json").write_text(json.dumps({
            "server": "http://127.0.0.1:9", "tenant": "demo"
        }))
        self.env = {key: value for key, value in os.environ.items() if key != "CJ_TOKEN"}
        self.env.update(XDG_CONFIG_HOME=str(base / "config"), XDG_STATE_HOME=str(base / "state"),
                        XDG_CACHE_HOME=str(base / "cache"), CJ_PROJECT="p",
                        DBUS_SESSION_BUS_ADDRESS=f"unix:path={base / 'missing-bus'}")

    def cli(self, *args, **env):
        return subprocess.run([self.binary, *args], env=dict(self.env, **env),
                              capture_output=True, text=True, timeout=10)

    def test_unreachable_keyring_is_not_reported_as_logged_out(self):
        result = self.cli("search", "x")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("system keyring", result.stderr)
        self.assertNotIn("run cj login", result.stderr)

    def test_stored_token_is_not_sent_to_another_server(self):
        result = self.cli("search", "x", "--server", "https://example.test")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("stored token belongs to http://127.0.0.1:9", result.stderr)

    def test_same_server_with_trailing_slash_uses_the_stored_token(self):
        result = self.cli("search", "x", "--server", "http://127.0.0.1:9/")
        self.assertIn("system keyring", result.stderr)

    def test_explicit_token_may_target_another_server(self):
        result = self.cli("search", "x", "--offline", "--server", "https://example.test",
                          CJ_TOKEN="test-token")
        self.assertNotIn("stored token belongs", result.stderr)


if __name__ == "__main__":
    unittest.main()
