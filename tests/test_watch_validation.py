import json
from feedback_audit_support import AuditCase, Handler


class WatchValidationTest(AuditCase):
    def test_invalid_watch_input_fails_before_credentials_network_or_execution(self):
        self.env.pop("CJ_TOKEN")
        self.config.unlink()
        marker = self.base / "executed"
        for arguments, message in [
            (["sh", "-c", f"touch {marker}; #" + "x" * 2001], "item 3"),
            (["true"] * 101, "100 arguments"),
            (["", "true"], "empty executable"),
        ]:
            result = self.cli("watch", "start", "--title", "Invalid", "--", *arguments)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(message, result.stderr)
            self.assertNotIn("x" * 100, result.stderr)
        for timeout in ["0", "25h", "86401", "1.5h", "18446744073709551615h"]:
            result = self.cli("watch", "start", "--title", "Invalid", "--timeout", timeout, "--", "true")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("1-86400 seconds", result.stderr)
        self.assertFalse(marker.exists())
        self.assertEqual(Handler.calls, [])

    def test_valid_unicode_boundary_and_duration_reach_the_api(self):
        # Reject the synthetic request remotely so no detached worker is started.
        Handler.failed = True
        result = self.cli("--project", "fixture", "watch", "start", "--title", "Boundary",
                          "--timeout", "3h", "--", "true", "è" * 2000)
        self.assertIn("queued", result.stdout.lower())
        request = next(body for method, path, body in Handler.calls if method == "POST" and path.endswith("/watches"))
        self.assertEqual(request["timeout"], 10800)
        self.assertEqual(len(request["command"][1]), 2000)
        help_text = self.cli("watch", "start", "--help").stdout
        for phrase in ["seconds", "3h", "2000 characters", "100 items", "script file"]:
            self.assertIn(phrase, help_text)

    def test_keyring_failure_reports_bounded_retries_without_claiming_missing_login(self):
        self.env.pop("CJ_TOKEN")
        self.env["DBUS_SESSION_BUS_ADDRESS"] = f"unix:path={self.base}/missing-bus"
        result = self.cli("--project", "fixture", "search", "synthetic")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("platform failures were retried twice", result.stderr)
        self.assertIn("does not mean the token is missing", result.stderr)
        self.assertEqual(Handler.calls, [])
