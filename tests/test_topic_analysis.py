import json
from unittest.mock import patch

from feedback_audit_support import AuditCase, Handler


ANALYSIS = {"truncated": True, "analyzed_topics": 500, "comparisons": 10000}


def read_response(handler):
    if handler.path.endswith("/topics/similar"):
        handler.reply(200, {"certain": [], "possible": [], "topic_analysis": ANALYSIS})
    elif handler.path.endswith("/garden/report"):
        handler.reply(200, {"topic_groups": [], "possible_topics": [], "topic_analysis": ANALYSIS})
    else:
        handler.reply(200, {"project": {"slug": "fixture"}, "entries": [], "docs": [], "next_page": None})


def maintenance_response(handler):
    handler.rfile.read(int(handler.headers["Content-Length"]))
    handler.reply(200, {"applied": [], "topic_analysis": ANALYSIS})


class TopicAnalysisTest(AuditCase):
    def test_similarity_and_garden_preserve_partial_metadata_and_explain_it(self):
        with patch.object(Handler, "do_GET", read_response):
            for command in [("topics", "similar"), ("garden", "--dry-run")]:
                result = self.cli("--json", *command, "--project", "fixture")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(result.stdout)["topic_analysis"], ANALYSIS)
                text = self.cli(*command, "--project", "fixture")
                self.assertEqual(text.returncode, 0, text.stderr)
                self.assertIn("Topic analysis is partial", text.stdout)
                self.assertIn("cj topics merge", text.stdout)

    def test_maintenance_does_not_lose_a_newly_partial_analysis(self):
        def complete_report(handler):
            handler.reply(200, {"project": {"slug": "fixture"}, "entries": [], "docs": [],
                                "topic_groups": [], "possible_topics": [], "next_page": None})

        with patch.object(Handler, "do_GET", complete_report), patch.object(Handler, "do_POST", maintenance_response):
            result = self.cli("--json", "garden", "--project", "fixture")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["topic_analysis"], ANALYSIS)
