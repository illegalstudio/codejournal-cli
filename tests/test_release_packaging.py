import hashlib
import json
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class LinuxReleasePackagingTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='cj-release-package-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for directory in ('target/mock/release', 'target/bundled-bwrap/sources', 'scripts/sandbox', 'licenses'):
            (self.root / directory).mkdir(parents=True, exist_ok=True)
        (self.root / 'Cargo.toml').write_text('[package]\nversion="0.1.0"\n')
        (self.root / 'LICENSE').write_text('Synthetic client license')
        (self.root / 'licenses/component.txt').write_text('Synthetic component notice')
        self.info = {'available': True, 'architecture': 'x86_64', 'version': '0.13.1'}
        self.binary()
        manifest = {}
        for name, algorithm in [('bubblewrap', 'sha256'), ('libcap', 'sha512')]:
            data = ('complete synthetic source '+name).encode()
            filename = name+'.tar.xz'
            (self.root / 'target/bundled-bwrap/sources' / filename).write_bytes(data)
            manifest[name] = {'version': '0.13.1', 'archive': filename, algorithm: hashlib.new(algorithm, data).hexdigest()}
        (self.root / 'scripts/sandbox/sources.json').write_text(json.dumps(manifest))
        (self.root / 'scripts/sandbox/container.sh').write_text('Synthetic build recipe')

    def binary(self):
        executable = self.root / 'target/mock/release/cj'
        executable.write_text(f'#!{sys.executable}\nimport json\nprint(json.dumps({self.info!r}))\n')
        executable.chmod(0o755)

    def package(self):
        return subprocess.run([sys.executable, str(ROOT / 'scripts/release/package.py'),
            'mock', 'linux-x64'], cwd=self.root, text=True, capture_output=True)

    def test_linux_archive_preserves_single_binary_and_complete_vendor_sources(self):
        result = self.package()
        self.assertEqual(result.returncode, 0, result.stderr)
        with tarfile.open(self.root / 'dist/codejournal-cli-v0.1.0-linux-x64.tar.gz') as archive:
            names = set(archive.getnames())
            for expected in ('cj', 'LICENSE', 'third-party/licenses/component.txt',
                             'third-party/sources/bubblewrap.tar.xz', 'third-party/sources/libcap.tar.xz',
                             'third-party/build/sources.json', 'third-party/build/container.sh'):
                self.assertIn(expected, names)
            self.assertNotIn('bwrap', names)

    def test_linux_release_rejects_missing_or_wrong_helper_metadata(self):
        for changes in ({'available': False}, {'architecture': 'aarch64'}, {'version': '0.0.1'}):
            with self.subTest(changes=changes):
                original = self.info.copy()
                self.info.update(changes)
                self.binary()
                result = self.package()
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(list((self.root / 'dist').glob('*.tar.gz')))
                self.info = original

    def test_corrupt_vendor_source_prevents_release_packaging(self):
        (self.root / 'target/bundled-bwrap/sources/libcap.tar.xz').write_bytes(b'damaged source')
        result = self.package()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('source checksum mismatch', result.stderr)
        self.assertFalse(list((self.root / 'dist').glob('*.tar.gz')))
