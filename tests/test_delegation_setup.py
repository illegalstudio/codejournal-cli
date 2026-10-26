import json
import subprocess
from delegation_support import DelegationCase


class DelegationSetupTest(DelegationCase):
    def setup_client(self,*args,check=True):
        result=subprocess.run([str(self.client),"setup","agents","--agent","codex",*args],
            env=self.env,cwd=self.root,text=True,capture_output=True,timeout=10)
        if check and result.returncode:
            self.fail(result.stderr)
        return result

    def test_setup_installs_both_embedded_protocols_and_uninstalls_owned_files(self):
        response=json.loads(self.setup_client().stdout)["agents"][0]
        self.assertTrue(response["delegation"]["installed"])
        for name in ("code-journal-delegation","code-journal-guest"):
            path=self.root/"codex/skills"/name/"SKILL.md"
            self.assertTrue(path.is_file())
            guide=self.invoke("guide",*(["--guest"] if name=="code-journal-guest" else [])).stdout
            self.assertEqual(path.read_text().strip(),guide.strip())
        status=json.loads(self.setup_client("--status").stdout)["agents"][0]
        self.assertTrue(status["delegation"]["installed"])
        self.setup_client("--uninstall")
        self.assertFalse((self.root/"codex/skills/code-journal-guest/SKILL.md").exists())
        guide=self.invoke("guide","--guest").stdout
        self.assertIn("CJ_DELEGATION_ID",guide)

    def test_refresh_rejects_customized_protocol_before_changing_hooks(self):
        self.setup_client()
        path=self.root/"codex/skills/code-journal-guest/SKILL.md"
        customized=path.read_text()+"\nPersonal instructions\n"
        path.write_text(customized)
        settings=self.root/"codex/hooks.json"
        before=settings.read_bytes()
        result=self.setup_client("--refresh",check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn("customized",result.stderr)
        self.assertEqual(path.read_text(),customized)
        self.assertEqual(settings.read_bytes(),before)
