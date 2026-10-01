import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("release_metadata", ROOT / "scripts/release/metadata.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ReleaseMetadataTests(unittest.TestCase):
    def test_manifests_reference_exact_verified_archives(self):
        with tempfile.TemporaryDirectory(prefix="cj-metadata-") as temporary:
            directory = pathlib.Path(temporary)
            for platform in MODULE.PLATFORMS:
                suffix = ".zip" if platform.startswith("windows") else ".tar.gz"
                (directory / f"codejournal-cli-v0.1.0-{platform}{suffix}").write_bytes(platform.encode())
            MODULE.generate("0.1.0", directory)
            checksums = dict(line.split("  ", 1)[::-1] for line in (directory / "SHA256SUMS").read_text().splitlines())
            self.assertEqual(len(checksums), 7)
            for name, digest in checksums.items():
                self.assertEqual(digest, hashlib.sha256((directory / name).read_bytes()).hexdigest())
            formula = (directory / "codejournal-cli.rb").read_text()
            for platform in MODULE.PLATFORMS[:-1]:
                name = f"codejournal-cli-v0.1.0-{platform}.tar.gz"
                self.assertIn(f"/v0.1.0/{name}", formula)
                self.assertIn(checksums[name], formula)
            scoop = json.loads((directory / "codejournal-cli.json").read_text())
            self.assertEqual(scoop["bin"], "cj.exe")
            self.assertEqual(scoop["architecture"]["64bit"]["hash"], checksums["codejournal-cli-v0.1.0-windows-x64.zip"])

    def test_incomplete_release_cannot_generate_a_formula(self):
        with tempfile.TemporaryDirectory(prefix="cj-metadata-") as temporary:
            directory = pathlib.Path(temporary)
            with self.assertRaises(FileNotFoundError):
                MODULE.generate("0.1.0", directory)
            self.assertFalse((directory / "codejournal-cli.rb").exists())

    def test_release_version_cannot_inject_package_metadata(self):
        with tempfile.TemporaryDirectory(prefix="cj-metadata-") as temporary:
            with self.assertRaises(ValueError):
                MODULE.generate('0.1.0"; system("bad")', pathlib.Path(temporary))


if __name__ == "__main__":
    unittest.main()
