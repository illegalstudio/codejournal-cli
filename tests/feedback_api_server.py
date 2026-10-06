"""Synthetic API responses for CLI feedback regressions."""
import http.server
import json
import urllib.parse

PLAN_ID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"


class Handler(http.server.BaseHTTPRequestHandler):
    calls = []
    entry_response = None

    def log_message(self, *_args):
        pass

    def respond(self, status, payload):
        content = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(content)))
        self.end_headers()
        self.wfile.write(content)

    def do_GET(self):
        path = urllib.parse.urlsplit(self.path).path
        if path.endswith("/plans"):
            self.respond(200, {"plans": [{"id": PLAN_ID}]})
        elif path.endswith("/docs"):
            self.respond(200, {"docs": [{"id": PLAN_ID}]})
        elif path.endswith(f"/plans/{PLAN_ID}") or path.endswith("/plans/aaaaaaaa"):
            self.respond(200, {"plan": {"id": PLAN_ID, "body": "## Steps\n- [ ] ship"}, "revisions": []})
        elif path.endswith(f"/docs/{PLAN_ID}") or path.endswith("/docs/aaaaaaaa"):
            self.respond(200, {"doc": {"id": PLAN_ID, "body": "Documentation body"}, "revisions": []})
        elif self.path.endswith("/tasks?all=1"):
            self.respond(200, {"tasks": [{"id": PLAN_ID}]})
        elif path.endswith("/rules"):
            self.respond(200, {"rules": "- Run focused tests."})
        else:
            self.respond(404, {"error": "unknown route"})

    def record(self):
        size = int(self.headers.get("Content-Length", "0"))
        body = json.loads(self.rfile.read(size))
        self.calls.append((self.command, self.path, body))
        response = {"rules": body["rules"]} if self.path.endswith("/rules") else {"ok": True}
        if self.path.endswith("/entries") and self.entry_response is not None:
            response = self.entry_response
        self.respond(200, response)

    do_PATCH = record
    do_POST = record
    do_PUT = record
