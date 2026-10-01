import hashlib
import io
import os
import pathlib
import subprocess
import tarfile
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="cj-installer-")
        self.addCleanup(self.temporary.cleanup)
        self.directory = pathlib.Path(self.temporary.name)
        self.bin = self.directory / "bin"
        self.bin.mkdir()
        self.output = self.directory / "installed"
        self.output.mkdir()
        self.scratch = self.directory / "scratch"
        self.scratch.mkdir()
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", CJ_VERSION="0.1.0",
                        CJ_INSTALL_DIR=str(self.output), TMPDIR=str(self.scratch), FIXTURE=str(self.directory))
        self.executable("uname", '#!/bin/sh\ncase "$1" in -s) echo Linux;; -m) echo x86_64;; esac\n')
        self.executable("curl", '''#!/bin/sh
url= output=
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) output="$2"; shift 2;;
    https:*) url="$1"; shift;;
    *) shift;;
  esac
done
case "$url" in
  */codejournal-cli-v0.1.0-linux-x64.tar.gz) cp "$FIXTURE/archive.tar.gz" "$output";;
  */SHA256SUMS) cp "$FIXTURE/SHA256SUMS" "$output";;
  */releases/latest) printf '{"tag_name":"v0.1.0"}\n';;
  *) exit 9;;
esac
''')
        self.archive("0.1.0")

    def executable(self, name, body):
        path = self.bin / name
        path.write_text(body)
        path.chmod(0o755)

    def archive(self, version):
        payload = f'#!/bin/sh\nprintf "cj {version}\\n"\n'.encode()
        with tarfile.open(self.directory / "archive.tar.gz", "w:gz") as archive:
            info = tarfile.TarInfo("cj")
            info.size = len(payload)
            info.mode = 0o755
            archive.addfile(info, io.BytesIO(payload))
        digest = hashlib.sha256((self.directory / "archive.tar.gz").read_bytes()).hexdigest()
        (self.directory / "SHA256SUMS").write_text(f"{digest}  codejournal-cli-v0.1.0-linux-x64.tar.gz\n")

    def install(self):
        result = subprocess.run(["sh", str(ROOT / "install.sh")], env=self.env, capture_output=True, text=True)
        self.assertEqual(list(self.scratch.iterdir()), [])
        self.assertEqual(list(self.output.glob(".cj-install.*")), [])
        return result

    def test_installs_verified_executable_and_resolves_latest(self):
        del self.env["CJ_VERSION"]
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(subprocess.check_output([str(self.output / "cj"), "--version"], text=True).strip(), "cj 0.1.0")

    def test_corruption_never_replaces_existing_binary(self):
        previous = self.output / "cj"
        previous.write_text("previous binary")
        with (self.directory / "archive.tar.gz").open("ab") as archive:
            archive.write(b"corruption")
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Checksum mismatch", result.stderr)
        self.assertEqual(previous.read_text(), "previous binary")

    def test_checksum_does_not_allow_a_wrong_binary_version(self):
        self.archive("9.9.9")
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.output / "cj").exists())

    def test_ambiguous_checksum_is_rejected(self):
        checksum = self.directory / "SHA256SUMS"
        checksum.write_text(checksum.read_text() * 2)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.output / "cj").exists())


if __name__ == "__main__":
    unittest.main()
