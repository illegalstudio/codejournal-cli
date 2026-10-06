import json
from feedback_audit_support import AuditCase, Handler


class FeedbackValidationTest(AuditCase):
    def test_optional_toml_sections_never_panic(self):
        for cargo, python in [("[workspace]\nmembers = []\n", "[build-system]\nrequires = []\n"),
                              ('[package]\nname = "some-product"\n', '[project]\nname = "other-product"\n'),
                              ('[[bin]]\npath = "src/main.rs"\n', '[tool.poetry]\nname = "poetry-product"\n')]:
            (self.repo / "Cargo.toml").write_text(cargo)
            (self.repo / "pyproject.toml").write_text(python)
            result = self.cli("brief")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("panic", result.stderr)

    def test_limits_are_checked_before_credentials_or_network(self):
        self.env.pop("CJ_TOKEN")
        self.config.unlink()
        refs = [arg for i in range(31) for arg in ["--ref", f"path:src/module{i}"]]
        result = self.cli("doc", "update", "12345678", *refs)
        self.assertIn("30 references", result.stderr)
        body = self.base / "body.md"
        body.write_text("a" * 100001)
        result = self.cli("doc", "create", "--title", "Too large", "--body-file", str(body))
        self.assertIn("100000 characters", result.stderr)
        result = self.cli("plan", "update", "12345678", stdin="a" * 100001)
        self.assertIn("100000 characters", result.stderr)
        self.assertEqual(Handler.calls, [])
        self.assertFalse((self.base / "state/codejournal/requests").exists())

    def test_unicode_characters_are_not_counted_as_utf8_bytes(self):
        body = self.base / "body.md"
        body.write_text("è" * 100000)
        result = self.cli("doc", "create", "--title", "Unicode", "--body-file", str(body), "--project", "fixture")
        self.assertEqual(result.returncode, 0, result.stderr)
        for command in ["create", "update"]:
            help_text = self.cli("doc", command, "--help").stdout
            self.assertIn("100000 characters", help_text)
            self.assertIn("30 unique", help_text)

    def test_rules_empty_output_is_safe_for_pipelines_and_404_remains_an_error(self):
        Handler.rules = ""
        result = self.cli("rules", "show", "--project", "fixture")
        self.assertEqual(result.stdout.strip(), "")
        self.assertIn("No project rules", result.stderr)
        Handler.missing = True
        result = self.cli("rules", "show", "--project", "fixture")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cj projects", result.stderr)

    def test_project_rename_refreshes_default_resolution_including_rename_back(self):
        self.assertEqual(self.cli("brief").returncode, 0)
        for slug in ["renamed", "fixture"]:
            result = self.cli("project", "edit", "--slug", slug)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = self.cli("--json", "brief")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["project"]["slug"], slug)
            self.assertTrue(any(path.endswith(f"/projects/{slug}/brief") for _, path, _ in Handler.calls))
