import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from delegation_support import DelegationCase


PROBE = r'''
if os.environ.get("CJ_FAKE_BEHAVIOR") == "credential_probe":
    expectation = os.environ["CJ_FAKE_EXPECT_CREDENTIAL"]
    if expectation == "absent":
        assert "CJ_TOKEN" not in os.environ and "CJ_TOKEN_SERVER" not in os.environ
    else:
        assert os.environ["CJ_TOKEN"] == "synthetic-credential"
        if expectation == "forwarded":
            assert os.environ["CJ_TOKEN_SERVER"] == "https://journal.invalid"
        else:
            assert "CJ_TOKEN_SERVER" not in os.environ
        read = subprocess.run([client, "--server", os.environ["CJ_FAKE_FOREIGN_SERVER"],
            "--project", "synthetic", "recent", "--json"], capture_output=True, text=True)
        if expectation == "forwarded":
            assert read.returncode != 0 and "belongs to" in read.stderr, read.stderr
        else:
            assert read.returncode == 0, read.stderr
'''


class DelegationCredentialTest(DelegationCase):
    def setUp(self):
        super().setUp()
        self.requests = []
        requests = self.requests

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                requests.append(self.headers.get("Authorization"))
                body = json.dumps({"entries": []}).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        self.env["CJ_FAKE_FOREIGN_SERVER"] = f"http://127.0.0.1:{server.server_port}"
        config = self.root / "config/codejournal/config.json"
        config.parent.mkdir(parents=True)
        config.write_text(json.dumps({"server": "https://journal.invalid", "tenant": "synthetic",
                                     "fallback_token": "synthetic-credential"}))
        config.chmod(0o600)
        fake = self.bin / "claude"
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")',
                                                'publish("ready", "Protocol loaded")\n' + PROBE))

    def probe(self, expectation):
        self.env["CJ_FAKE_EXPECT_CREDENTIAL"] = expectation
        result = self.settled(self.start("credential_probe"))
        self.assertEqual(result["status"], "completed", json.dumps(result))

    def test_automatic_forwarding_keeps_the_server_binding(self):
        self.probe("forwarded")
        self.assertEqual(self.requests, [])

    def test_a_different_selected_server_does_not_receive_stored_credentials(self):
        self.env["CJ_SERVER_URL"] = self.env["CJ_FAKE_FOREIGN_SERVER"]
        self.probe("absent")
        self.assertEqual(self.requests, [])

    def test_explicit_user_tokens_keep_the_existing_override_behavior(self):
        self.env["CJ_TOKEN"] = "synthetic-credential"
        self.probe("explicit")
        self.assertEqual(self.requests, ["Bearer synthetic-credential"])
