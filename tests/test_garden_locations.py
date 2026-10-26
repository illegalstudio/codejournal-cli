from unittest.mock import patch

import test_garden_review as modern
from feedback_audit_support import AuditCase, Handler, RESOURCE


class GardenLocationTest(AuditCase):
    def test_source_locations_probe_and_hash_the_file_and_preserve_existing_literal_names(self):
        refs = ["README.md:42", "README.md:10-20", "literal:12", "../outside:30"]
        def read(handler):
            if '/garden?' in handler.path:
                handler.reply(200, {"entries": [{"id": RESOURCE, "refs": [{"kind": "path", "value": ref} for ref in refs]}], "docs": []})
            else:
                modern.read(handler)
        def snapshot():
            Handler.calls.clear()
            result = self.cli("--json", "--project", "fixture", "garden", "--dry-run")
            self.assertEqual(result.returncode, 0, result.stderr)
            return next(body for _, path, body in Handler.calls if path.endswith('/preview') and body.get('entries'))
        (self.repo / "README.md").write_text("Synthetic\n")
        (self.repo / "literal:12").write_text("Literal filename\n")
        with patch.object(Handler, 'do_GET', read), patch.object(Handler, 'do_POST', modern.write):
            before = snapshot()
            observed = {row['path']: row['present'] for row in before['observations']}
            self.assertEqual(observed, {ref: True for ref in refs[:-1]})
            (self.repo / "README.md").write_text("Relevant change\n")
            self.assertNotEqual(snapshot()['entries'][0]['code_hash'], before['entries'][0]['code_hash'])
            (self.repo / "literal:12").unlink()
            self.assertFalse(next(row for row in snapshot()['observations'] if row['path'] == 'literal:12')['present'])
