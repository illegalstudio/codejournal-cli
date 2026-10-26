import http.server
import json
import urllib.parse


class Handler(http.server.BaseHTTPRequestHandler):
    paths = []

    def log_message(self, *_args):
        pass

    def do_GET(self):
        self.paths.append(self.path)
        parsed = urllib.parse.urlsplit(self.path)
        query = urllib.parse.parse_qs(parsed.query)
        item = {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "title": "Current",
                "body": "Current body", "status": "done", "project_slug": "p",
                "revision": 60, "refs": [{"kind": "commit", "value": "abcdef123"}]}
        if parsed.path.endswith(("/docs/aaaaaaaa", "/plans/aaaaaaaa")):
            key = "doc" if "/docs/" in parsed.path else "plan"
            if "revision" in query:
                revision = int(query["revision"][0])
                if not 1 <= revision <= 60:
                    return self.respond(404, {"message": "Revision not found"})
                item.update(revision=revision, body=f"Previous body {revision}")
            before = int(query.get("history_before", [61])[0])
            revisions = [{"body": f"Previous body {number}" * 100,
                          "title": f"Version {number}", "revision": number}
                         for number in range(min(before - 1, 60), 0, -1)]
            cursor = None
            if query.get("history") == ["0"]:
                revisions = []
            elif "history_content" in query or "history_before" in query:
                if len(revisions) > 20:
                    cursor = revisions[19]["revision"]
                revisions = revisions[:20]
                if query.get("history_content") == ["0"]:
                    for row in revisions:
                        row.update(body="", refs=[], content_included=False)
            payload = {key: item, "revision_next": cursor, "revisions": revisions,
                       "revision_count": 60, "current_revision": 60}
            status = 200
        elif parsed.path.endswith("/logs/aaaaaaaa"):
            payload, status = {"log": item}, 200
        elif "/entries/" in parsed.path:
            payload, status = {"message": "Entry ID is missing or ambiguous", "trace": []}, 422
        elif "/brief" in parsed.path:
            payload, status = {"message": "Project not found", "trace": []}, 404
        else:
            payload, status = {"message": "Log not found", "exception": "PrivateClass",
                               "file": "/private/server.php", "trace": [{"secret": "hidden"}]}, 404
        self.respond(status, payload)

    def do_PATCH(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        self.respond(404, {"message": "Project not found", "trace": []})

    def respond(self, status, payload):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
