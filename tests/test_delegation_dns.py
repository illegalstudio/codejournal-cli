import os
import shutil
import socket
import subprocess
import sys
import unittest
from delegation_support import DelegationCase


PROBE = r'''
resolver = Path("/etc/resolv.conf")
assert resolver.read_text() == "nameserver 127.0.0.53\n# synthetic resolver\n"
assert not Path("/run/cj-dns-test/private.sock").exists()
assert not Path("/run/cj-dns-test/private.txt").exists()
try:
    resolver.write_text("damaged")
except OSError:
    pass
else:
    raise RuntimeError("Resolver is writable")
print("resolver available and runtime sockets hidden")
'''


@unittest.skipUnless(sys.platform == "linux", "Linux mount namespace regression")
class DelegationDnsTest(DelegationCase):
    def test_runtime_resolver_symlinks_remain_readable_without_exposing_runtime(self):
        outer = shutil.which("bwrap")
        self.assertIsNotNone(outer)
        etc = self.root / "etc"
        etc.mkdir()
        var = self.root / "var"
        var.mkdir()
        (var / "run").symlink_to("../run")
        cache = "/etc/ld.so.cache"
        if os.path.isfile(cache):
            shutil.copy2(cache, etc / "ld.so.cache")
        runtime = self.root / "host-runtime"
        runtime.mkdir()
        resolver = runtime / "resolv.conf"
        resolver.write_text("nameserver 127.0.0.53\n# synthetic resolver\n")
        shutil.copy2(resolver, etc / "static-resolv.conf")
        (runtime / "private.txt").write_text("unrelated runtime state")
        with socket.socket(socket.AF_UNIX) as private:
            private.bind(str(runtime / "private.sock"))
            for destination in ("/run/cj-dns-test/resolv.conf",
                                "../var/run/cj-dns-test/resolv.conf", "static-resolv.conf", None,
                                "/run/cj-dns-test"):
                link = etc / "resolv.conf"
                link.unlink(missing_ok=True)
                if destination is None:
                    shutil.copy2(resolver, link)
                else:
                    link.symlink_to(destination)
                for target in ("codex", "grok"):
                    with self.subTest(destination=destination, target=target):
                        fake = self.bin / target
                        fake.write_text(f"#!{sys.executable}\nfrom pathlib import Path\n" + PROBE)
                        command = [outer, "--unshare-user", "--unshare-pid", "--die-with-parent",
                            "--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc",
                            "--tmpfs", "/run", "--ro-bind", str(etc), "/etc",
                            "--ro-bind", str(var), "/var",
                            "--ro-bind", str(runtime), "/run/cj-dns-test",
                            "--bind", str(self.root), str(self.root), "--", str(self.client),
                            "delegation", "run", target, "--prompt", "Inspect"]
                        result = subprocess.run(command, env=self.env, cwd=self.root,
                            text=True, capture_output=True, timeout=15)
                        if destination == "/run/cj-dns-test":
                            self.assertNotEqual(result.returncode, 0)
                            self.assertIn("DNS configuration must be a regular file", result.stderr)
                            continue
                        self.assertEqual(result.returncode, 0, result.stderr)
                        self.assertIn("resolver available and runtime sockets hidden", result.stdout)
        self.assertEqual(resolver.read_text(), "nameserver 127.0.0.53\n# synthetic resolver\n")
