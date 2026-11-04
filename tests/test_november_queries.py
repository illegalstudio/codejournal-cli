import json
import urllib.parse
from unittest.mock import patch
from feedback_audit_support import AuditCase, Handler, RESOURCE


def read(handler):
    Handler.calls.append(("GET", handler.path, None))
    path = urllib.parse.urlparse(handler.path).path
    query = urllib.parse.parse_qs(urllib.parse.urlparse(handler.path).query)
    if "/entries/" in path:
        return handler.reply(422, {"message":"Entry ID is missing or ambiguous"})
    if "/records/" in path:
        return handler.reply(200, {"matches":[{"type":"task","id":RESOURCE,"command":f"cj task show {RESOURCE}"}]})
    if "/docs/" in path:
        return handler.reply(200, {"doc":{"id":RESOURCE,"body":"Canonical content","revision":1,"refs":'[{"kind":"path","value":"src/app.rs"}]'}})
    handler.reply(200, {"project":{"slug":"fixture"},"entries":[],"documents":[],
        "reference_filter":{"kind":query.get("ref_kind", [""])[0],"value":query.get("ref_value", [""])[0]},
        "relaxed":bool(query.get("relaxed")),"tasks":[],"grep_mode":"whole_word","notifications":[],
        "grep_applied":True,"unread":0,"topics":[{"name":"php-ext","count":2}],"filter_applied":True})


def change(handler):
    body = json.loads(handler.rfile.read(int(handler.headers["Content-Length"])))
    Handler.calls.append(("PATCH", handler.path, body))
    if handler.path.endswith("/refs"):
        return handler.reply(200, {"entry":{"id":RESOURCE,"refs":body["refs"],"refs_version":"b"*64}})
    handler.reply(200, {"log":{"id":RESOURCE,"refs":body.get("refs",[]),"revision":1}})


class NovemberQueriesTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.env["CODE_JOURNAL_AUTO_SYNC"] = "off"
        self.enterContext(patch.object(Handler, "do_GET", read))
        self.enterContext(patch.object(Handler, "do_PATCH", change))

    def test_reference_and_relaxed_queries_preserve_explicit_scope(self):
        result = self.cli("--project","fixture","search","--ref","acme/compiler#2118","--json")
        self.assertEqual(result.returncode,0,result.stderr)
        query = urllib.parse.parse_qs(urllib.parse.urlparse(Handler.calls[-1][1]).query)
        self.assertEqual(query["ref_kind"],["url"])
        self.assertEqual(query["ref_value"],["https://github.com/acme/compiler/issues/2118"])
        self.assertEqual(query["project"],["fixture"])
        result = self.cli("--project","fixture","search","loose comparison","--relaxed","--json")
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertTrue(json.loads(result.stdout)["relaxed"])

    def test_task_notification_and_topic_filters_reach_the_server(self):
        for args, key, value in [
            (("task","list","--grep","INI","--whole-word"),"whole_word","1"),
            (("notifications","list","--grep","specific approval"),"grep","specific approval"),
            (("topics","find","php-ext"),"grep","php-ext"),
        ]:
            result = self.cli("--project","fixture",*args,"--json")
            self.assertEqual(result.returncode,0,result.stderr)
            query = urllib.parse.parse_qs(urllib.parse.urlparse(Handler.calls[-1][1]).query)
            self.assertEqual(query[key],[value])

    def test_ref_only_entry_corrections_keep_the_original_protected_base(self):
        result = self.cli("entry","refs",RESOURCE,"--base","a"*64,"--ref","path:src/correct.rs","--json")
        self.assertEqual(result.returncode,0,result.stderr)
        payload = Handler.calls[-1][2]
        self.assertEqual(payload,{"based_on":"a"*64,"refs":[{"kind":"path","value":"src/correct.rs"}]})

    def test_json_refs_and_metadata_only_log_confirmations_are_explicit(self):
        result = self.cli("doc","show",RESOURCE,"--json")
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(json.loads(result.stdout)["doc"]["refs"],[{"kind":"path","value":"src/app.rs"}])
        result = self.cli("log","update",RESOURCE,"--ref","path:src/app.rs","--json")
        self.assertEqual(result.returncode,0,result.stderr)
        payload = json.loads(result.stdout)
        self.assertTrue(payload["body_omitted"])
        self.assertEqual(payload["readback_command"],f"cj log show {RESOURCE} --json")

    def test_bare_id_errors_name_the_matching_record_type(self):
        result = self.cli("show",RESOURCE)
        self.assertNotEqual(result.returncode,0)
        self.assertIn(f"cj task show {RESOURCE}",result.stderr)
