import json
from feedback_audit_support import Handler


class Plans:
    def __init__(self):
        self.items = {}
        self.receipts = {}
        self.attempts = []
        self.uncertain = False
        self.conflict = False
        self.read_original = Handler.do_GET

    def read(self, handler):
        path = handler.path.split("?")[0]
        for ident, item in self.items.items():
            if path.endswith("/plans/" + ident):
                return handler.reply(200, {"plan": item})
        return self.read_original(handler)

    def write(self, handler):
        body = json.loads(handler.rfile.read(int(handler.headers["Content-Length"])))
        key = handler.headers.get("Idempotency-Key")
        self.attempts.append((key, handler.command, body))
        if key in self.receipts:
            return handler.reply(201 if handler.command == "POST" else 200, self.receipts[key])
        if handler.command == "POST":
            item = dict(body, revision=1, project_slug="fixture")
            self.items[body["id"]] = item
        else:
            item = self.items[handler.path.rsplit("/", 1)[-1]]
            if self.conflict or (body.get("action") == "step" and body["based_on"] != item["revision"]):
                return handler.reply(409, {"message": "Plan changed"})
            if body.get("action") == "step":
                lines = item["body"].splitlines()
                at = body["step_index"] - 1
                lines[at] = lines[at].replace("[ ]", "[x]") if body["step_done"] else lines[at].replace("[x]", "[ ]")
                item["body"] = "\n".join(lines)
            else:
                item.update({key: value for key, value in body.items() if key in ["body", "title", "status"]})
            item["revision"] += 1
        response = {"plan": dict(item)}
        self.receipts[key] = response
        if self.uncertain:
            self.uncertain = False
            handler.send_response(201)
            handler.send_header("Content-Length", "4")
            handler.end_headers()
            return handler.wfile.write(b"html")
        handler.reply(201 if handler.command == "POST" else 200, response)
