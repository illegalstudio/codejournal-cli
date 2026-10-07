import http.server
import json
import urllib.parse
import uuid


class Handler(http.server.BaseHTTPRequestHandler):
    watches = {}
    notifications = []
    receipts = {}
    paths = []
    override = None
    fail_creation = False
    fail_detail = False
    fail_listing = False
    fail_result = False
    accepted_nonjson = False
    heartbeats = 0
    heartbeat_override = None

    @classmethod
    def reset(cls):
        cls.watches.clear()
        cls.notifications.clear()
        cls.receipts.clear()
        cls.paths.clear()
        cls.override = None
        cls.fail_creation = cls.fail_detail = cls.fail_listing = cls.fail_result = False
        cls.accepted_nonjson = False
        cls.heartbeats = 0
        cls.heartbeat_override = None

    def log_message(self, *_args):
        pass

    def respond(self, payload, status=200):
        self.send_data(json.dumps(payload).encode(), status, "application/json")

    def send_data(self, data, status, content_type):
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Content-Type", content_type)
        self.end_headers()
        self.wfile.write(data)

    def body(self):
        return json.loads(self.rfile.read(int(self.headers["Content-Length"])))

    def do_POST(self):
        body = self.body()
        self.paths.append(("POST", self.path, self.headers.get("Idempotency-Key")))
        if self.path.endswith("/heartbeat"):
            watch = self.watches[self.path.split("/watches/")[1].split("/")[0]]
            active = watch["status"] in ["starting", "running", "lost"]
            if active:
                watch.update(status="running", runner_id=body["runner_id"], lease_expires_at="renewed")
            type(self).heartbeats += 1
            self.respond(self.heartbeat_override if self.heartbeat_override is not None else {"watch": watch, "active": active})
            return
        if self.fail_creation:
            self.respond({"message": "Synthetic temporary outage"}, 503)
            return
        key = self.headers.get("Idempotency-Key")
        if key in self.receipts:
            self.respond(self.receipts[key], 201)
            return
        ident = body.get("id", str(uuid.uuid4()))
        slug = self.path.split("/projects/")[1].split("/")[0]
        watch = dict(body, id=ident, project_slug=slug, command=json.dumps(body["command"]),
                     status="starting" if body.get("managed") else "running",
                     lease_expires_at="initial" if body.get("managed") else None)
        self.watches[ident] = watch
        self.receipts[key] = {"watch": dict(watch)}
        if self.accepted_nonjson:
            type(self).accepted_nonjson = False
            self.send_data(b"<html>Private synthetic proxy error</html>", 201, "text/html")
            return
        self.respond({"watch": watch}, 201)

    def do_GET(self):
        self.paths.append(("GET", self.path, None))
        parsed = urllib.parse.urlsplit(self.path)
        query = urllib.parse.parse_qs(parsed.query)
        if parsed.path.endswith("/me"):
            self.respond({"user": {"id": 1}})
        elif parsed.path.endswith("/notifications"):
            self.respond({"notifications": self.notifications})
        elif "/watches/" in parsed.path:
            if self.fail_detail:
                self.send_data(b"<html>Private synthetic proxy error</html>", 503, "text/html")
                return
            prefix = parsed.path.rsplit("/", 1)[-1]
            matches = [watch for ident, watch in self.watches.items() if ident.startswith(prefix)
                       and ("host" not in query or watch.get("host") == query["host"][0])]
            if len(matches) != 1:
                self.respond({"message": "Watch missing or ambiguous"}, 404 if not matches else 422)
                return
            watch = dict(matches[0])
            if self.override:
                watch.update(self.override)
            self.respond({"watch": watch})
        else:
            if self.fail_listing:
                self.respond({"message": "Synthetic history unavailable"}, 503)
                return
            watches = list(self.watches.values())
            if query.get("all") != ["1"]:
                watches = [watch for watch in watches if watch["status"] in ["starting", "running"]]
            self.respond({"watches": watches[-100:]})

    def do_PATCH(self):
        body = self.body()
        self.paths.append(("PATCH", self.path, self.headers.get("Idempotency-Key")))
        if self.fail_result:
            self.respond({"message": "Synthetic result delivery outage"}, 503)
            return
        watch = self.watches[self.path.rsplit("/", 1)[-1]]
        if watch["status"] not in ["starting", "running", "lost"] or watch["status"] == body["status"]:
            self.respond({"watch": watch, "changed": False})
            return
        watch.update(body)
        failed = body["status"] in ["timed_out", "lost"] or body.get("exit_code", 0) != 0
        if body["status"] != "cancelled" and (watch.get("notify_on") != "failure" or failed):
            outcome = "timed out" if body["status"] == "timed_out" else "failed" if failed else "succeeded"
            self.notifications.append({"title": f"{watch['title']}: {outcome}"})
        self.respond({"watch": watch, "changed": True})
