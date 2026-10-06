import json
import shlex
import subprocess
from unittest.mock import patch

from feedback_audit_support import AuditCase, Handler


class TopicCommandsTest(AuditCase):
    def test_suggestions_preserve_untrusted_names_without_shell_execution(self):
        target = "$(printf CJ_INJECTED >&2)' target"
        sources = ["`printf CJ_INJECTED >&2`", "--server=evil", "semi; printf CJ_INJECTED >&2"]

        def respond(handler):
            handler.reply(200, {"certain": [[target, *sources]], "possible": []})

        with patch.object(Handler, "do_GET", respond):
            result = self.cli("topics", "similar", "--project", "fixture")
            structured = self.cli("--json", "topics", "similar", "--project", "fixture")
        self.assertEqual(result.returncode, 0, result.stderr)
        command = next(line.strip() for line in result.stdout.splitlines() if line.startswith("  cj topics merge"))
        self.assertEqual(shlex.split(command), ["cj", "topics", "merge", "--into=" + target, "--", *sources])
        script = 'cj() { python3 -c \'import json,sys; print(json.dumps(sys.argv[1:]))\' "$@"; };\n' + command
        executed = subprocess.run(["sh", "-c", script], text=True, capture_output=True, timeout=5)
        self.assertEqual(executed.returncode, 0, executed.stderr)
        self.assertEqual(executed.stderr, "")
        self.assertEqual(json.loads(executed.stdout), ["topics", "merge", "--into=" + target, "--", *sources])
        self.assertEqual(structured.returncode, 0, structured.stderr)
        self.assertEqual(json.loads(structured.stdout)["certain"], [[target, *sources]])
