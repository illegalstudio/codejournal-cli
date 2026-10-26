import json
import time
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


def raw(handler, status, content_type="text/html", body=b"Private synthetic proxy contents"):
    Handler.calls.append((handler.command, handler.path, None))
    handler.send_response(status)
    handler.send_header("Content-Type", content_type)
    handler.send_header("Content-Length", str(len(body)))
    handler.end_headers()
    try:
        handler.wfile.write(body)
    except BrokenPipeError:
        pass


class ReadDiagnosticsTest(AuditCase):
    def test_invalid_json_reads_have_safe_http_and_endpoint_diagnostics(self):
        commands = [("sync",), ("plan", "list"), ("doc", "show", RESOURCE),
                    ("plan", "show", RESOURCE), ("search", "synthetic"), ("log", "show", RESOURCE)]
        for command in commands:
            with self.subTest(command=command), patch.object(Handler, "do_GET", lambda h: raw(h, 200)):
                result = self.cli("--project", "fixture", *command)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("HTTP 200", result.stderr)
                self.assertIn("Content-Type text/html", result.stderr)
                self.assertIn("GET http://127.0.0.1", result.stderr)
                self.assertNotIn("Private synthetic", result.stderr)
                self.assertNotIn("synthetic", result.stderr.split("GET ")[-1].split(")")[0])

    def test_empty_404_explains_configuration_registration_and_endpoint_compatibility(self):
        with patch.object(Handler, "do_POST", lambda h: raw(h, 404, body=b"")):
            result = self.cli("--project", "fixture", "brief")
        self.assertNotEqual(result.returncode, 0)
        for detail in ["HTTP 404", "Content-Type text/html", "/projects/fixture/brief", "cj status", "cj projects", "endpoint"]:
            self.assertIn(detail, result.stderr)
        self.assertEqual(len(Handler.calls), 1)

    def test_invalid_response_preserves_cached_document_and_labels_it(self):
        response = {"doc": {"id": RESOURCE, "title": "Useful guide", "body": "Known content", "revision": 1}}
        with patch.object(Handler, "do_GET", lambda h: h.reply(200, response)):
            self.assertEqual(self.cli("--json", "doc", "show", RESOURCE).returncode, 0)
        with patch.object(Handler, "do_GET", lambda h: raw(h, 502)):
            result = self.cli("--json", "doc", "show", RESOURCE)
        self.assertEqual(result.returncode, 0, result.stderr)
        cached = json.loads(result.stdout)
        self.assertEqual(cached["doc"], response["doc"])
        self.assertTrue(cached["cached"])
        self.assertIn("HTTP 502", result.stderr)
        self.assertIn("may be outdated", result.stderr)
        with patch.object(Handler, "do_GET", lambda h: raw(h, 403)):
            refused = self.cli("--json", "doc", "show", RESOURCE)
        self.assertNotEqual(refused.returncode, 0)
        self.assertEqual(refused.stdout, "")

    def test_uncached_search_failure_is_not_an_empty_result_or_delivered_write(self):
        with patch.object(Handler, "do_GET", lambda h: raw(h, 503)):
            result = self.cli("--project", "fixture", "search", "synthetic")
        self.assertNotEqual(result.returncode, 0)
        for detail in ["No cached response", "context is unavailable", "--offline", "not synchronized"]:
            self.assertIn(detail, result.stderr)
        queued = self.cli("--json", "--offline", "--project", "fixture", "add", "--kind", "gotcha",
                          "--title", "Outage fact", "--body", "Synthetic")
        self.assertTrue(json.loads(queued.stdout)["queued"])

    def test_sessionless_brief_omits_the_optional_session_field(self):
        result = self.cli("--json", "--project", "fixture", "brief")
        self.assertEqual(result.returncode, 0, result.stderr)
        body = next(body for method, path, body in Handler.calls if method == "POST" and path.endswith("/brief"))
        self.assertNotIn("session", body)

    def test_first_brief_times_out_once_and_reports_unavailable_context(self):
        def slow(handler):
            handler.rfile.read(int(handler.headers["Content-Length"]))
            time.sleep(5.5)
            raw(handler, 503)
        start = time.monotonic()
        with patch.object(Handler, "do_POST", slow):
            result = self.cli("--project", "fixture", "brief")
        self.assertLess(time.monotonic() - start, 6.5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("No cached response", result.stderr)
        self.assertIn("not synchronized", result.stderr)
