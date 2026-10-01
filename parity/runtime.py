"""Isolated DDEV database and local HTTP server for differential CLI tests."""
import json
import os
import pathlib
import re
import signal
import shlex
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[3]
SERVER = ROOT / "apps" / "server"


def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, **kwargs)


def bootstrap():
    description = json.loads(run("ddev", "describe", "-j", capture_output=True).stdout)["raw"]
    port = description["services"]["db"]["host_ports"]
    # Capture the existing local runtime password in memory, never print or persist it.
    php = "require 'vendor/autoload.php'; $app=require 'bootstrap/app.php'; $app->make(Illuminate\\Contracts\\Console\\Kernel::class)->bootstrap(); echo config('database.connections.journal.password');"
    password = subprocess.check_output(
        ["php", "-r", php], cwd=SERVER, text=True)
    if not password:
        password = run("ddev", "exec", "-d", "/var/www/html/apps/server",
                       "php -r " + shlex.quote(php), capture_output=True).stdout
    if not password:
        raise RuntimeError("Configure the local DDEV journal runtime role before parity tests")
    name = f"cj_parity_{os.getpid()}"
    env = dict(os.environ, APP_ENV="testing", APP_DEBUG="false", DB_CONNECTION="pgsql", DB_HOST="127.0.0.1",
               DB_PORT=port, DB_DATABASE=name, DB_USERNAME="db", DB_PASSWORD="db", CACHE_STORE="array",
               JOURNAL_DB_USERNAME="journal_app", JOURNAL_DB_PASSWORD=password, JOURNAL_STORAGE="postgres",
               SESSION_DRIVER="array", QUEUE_CONNECTION="sync", BROADCAST_CONNECTION="null", LOG_CHANNEL="stderr")
    return name, env


class Runtime:
    def __enter__(self):
        self.name, self.env = bootstrap()
        self.temp = tempfile.TemporaryDirectory(prefix=".cli-parity-", dir=ROOT)
        self.folder = pathlib.Path(self.temp.name)
        self.process = None
        self.created = False
        try:
            run("ddev", "exec", "-s", "db", "createdb", "-U", "db", self.name, capture_output=True)
            self.created = True
            subprocess.run(["php", "artisan", "migrate", "--force"], cwd=SERVER, env=self.env,
                           check=True, capture_output=True)
            source = (SERVER / "app/Console/Commands/SetupLocalJournalRole.php").read_text()
            grants = ";\n".join(re.findall(r"DB::statement\('(GRANT[^']+)'\);", source)) + ";\n"
            run("ddev", "exec", "-s", "db", "psql", "-v", "ON_ERROR_STOP=1", "-U", "db", "-d", self.name,
                input=grants, capture_output=True)
            with socket.socket() as sock:
                sock.bind(("127.0.0.1", 0))
                port = sock.getsockname()[1]
            self.url = f"http://127.0.0.1:{port}"
            self.log = open(self.folder / "http.log", "w+")
            router = SERVER / "vendor/laravel/framework/src/Illuminate/Foundation/resources/server.php"
            self.process = subprocess.Popen(["php", "-S", f"127.0.0.1:{port}", "-t", str(SERVER / "public"), str(router)],
                                            cwd=SERVER / "public", env=self.env, stdout=self.log, stderr=self.log,
                                            start_new_session=True)
            for _ in range(100):
                try:
                    urllib.request.urlopen(self.url + "/up", timeout=1).close()
                    return self
                except (urllib.error.URLError, TimeoutError):
                    if self.process.poll() is not None:
                        raise RuntimeError("Parity HTTP server exited")
                    time.sleep(0.05)
            raise RuntimeError("Parity HTTP server did not start")
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def fixture(self, cleanup=None):
        env = dict(self.env)
        if cleanup:
            env["CJ_PARITY_CLEANUP"] = cleanup
        result = subprocess.run(["php", "tests/Support/seed-cli-parity.php"], cwd=SERVER, env=env,
                                check=True, capture_output=True, text=True)
        return None if cleanup else json.loads(result.stdout)

    def __exit__(self, *_args):
        if self.process is not None and self.process.poll() is None:
            os.killpg(self.process.pid, signal.SIGTERM)
            self.process.wait(timeout=5)
        if hasattr(self, "log"):
            self.log.close()
        try:
            if self.created:
                run("ddev", "exec", "-s", "db", "dropdb", "-U", "db", self.name, capture_output=True)
        finally:
            self.temp.cleanup()
