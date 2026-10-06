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
                "revision": 2, "refs": [{"kind": "commit", "value": "abcdef123"}]}
        if parsed.path.endswith(("/docs/aaaaaaaa", "/plans/aaaaaaaa")):
            key = "doc" if "/docs/" in parsed.path else "plan"
            payload = {key: item, "revision_next": None if query.get("history") == ["0"] else 1, "revisions": [] if query.get("history") == ["0"] else
                       [{"body": "Previous body" * 100, "revision": 1}]}
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
