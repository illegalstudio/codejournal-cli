"""Paired command processes, each with its own state and synthetic journal."""
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
from runtime import ROOT

RUNTIME = None
REFERENCE = None


class ParityCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="case-", dir=RUNTIME.folder)
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.fixture = RUNTIME.fixture()
        self.addCleanup(RUNTIME.fixture, self.fixture["tenant"])
        self.repo = self.base / "repo"
        self.repo.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Parity Tester")
        self.git("config", "user.email", "parity@example.test")
        self.git("remote", "add", "origin", "https://example.test/parity/repo.git")
        (self.repo / "README.md").write_text("Synthetic repository\n")
        self.git("add", "README.md")
        self.git("commit", "-m", "Synthetic initial commit")
        self.envs = {}
        for engine in ["python", "rust"]:
            home = self.base / engine
            home.mkdir()
            config = home / "config" / ("code-journal" if engine == "python" else "codejournal")
            config.mkdir(parents=True)
            (config / "config.json").write_text("{}" if engine == "python" else json.dumps({
                "server": RUNTIME.url, "tenant": self.fixture["tenant"], "notifications": {"desktop": False}}))
            env = dict(os.environ, XDG_CONFIG_HOME=str(home / "config"), XDG_STATE_HOME=str(home / "state"),
                       XDG_CACHE_HOME=str(home / "cache"), XDG_DATA_HOME=str(home / "data"),
                       HOME=str(home), CODEX_HOME=str(home / ".codex"), CJ_TOKEN=self.fixture["token"],
                       CODE_JOURNAL_BACKEND="local", CODE_JOURNAL_DB_PATH=str(home / "journal.sqlite"),
                       CODE_JOURNAL_HOST="parity-host", GIT_CONFIG_GLOBAL=os.devnull,
                       GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="commit.gpgsign",
                       GIT_CONFIG_VALUE_0="false", GIT_AUTHOR_NAME="Parity Tester", GIT_AUTHOR_EMAIL="parity@example.test",
                       GIT_COMMITTER_NAME="Parity Tester", GIT_COMMITTER_EMAIL="parity@example.test")
            for key in ["CJ_SERVER_URL", "CJ_PROJECT", "ME_AI_JOURNAL_SYNC_URL", "CODE_JOURNAL_SYNC_URL",
                        "ME_AI_JOURNAL_AUTH_TOKEN", "CODE_JOURNAL_AUTH_TOKEN", "CJ_HOOK_AGENT", "ME_DELEGATION_ID"]:
                env.pop(key, None)
            for key in ["CJ_SESSION_ID", "CLAUDE_CODE_SESSION_ID", "CODEX_THREAD_ID", "CURSOR_SESSION_ID",
                        "CODE_JOURNAL_AGENT", "ME_AI_JOURNAL_AGENT", "CURSOR_VERSION", "ME_DELEGATION_ROLE"]:
                env.pop(key, None)
            self.envs[engine] = env
        self.invoke("python", "rules", "set", "Use synthetic data.", project=None)
        self.invoke("rust", "project", "init", project=None)
        self.invoke("rust", "rules", "set", "Use synthetic data.")

    def git(self, *args):
        return subprocess.run(["git", "-c", "commit.gpgsign=false", *args], cwd=self.repo, check=True,
                              capture_output=True, text=True).stdout.strip()

    def invoke(self, engine, *args, json_mode=True, input=None, check=True, project="parity-repo", cwd=None):
        command = [sys.executable, str(REFERENCE)] if engine == "python" else [str(ROOT / "apps/cli/target/debug/cj")]
        if project is not None:
            command += ["--project", project]
        if json_mode:
            command += ["--json"]
        result = subprocess.run([*command, *map(str, args)], cwd=cwd or self.repo, env=self.envs[engine],
                                input=input, capture_output=True, text=True, timeout=15)
        if check and result.returncode:
            self.fail(f"{engine} {' '.join(map(str, args))}: {result.stderr}")
        return json.loads(result.stdout) if json_mode and check else result

    def paired(self, *args, **kwargs):
        return {engine: self.invoke(engine, *args, **kwargs) for engine in self.envs}

    def add_entry(self, title="Cache invalidation", **kwargs):
        return {engine: self.invoke(engine, "add", "--kind", "gotcha", "--title", title,
                                  "--body", "Synthetic cache knowledge", "--topics", "cache", **kwargs)
                for engine in self.envs}

    def titles(self, pair, key, expected):
        for engine, result in pair.items():
            self.assertEqual(sorted(item["title"] for item in result[key]), sorted(expected), engine)
