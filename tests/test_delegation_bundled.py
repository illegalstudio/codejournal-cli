import hashlib
import json
import shutil
import subprocess
import sys
import unittest
from pathlib import Path
from delegation_support import DelegationCase


@unittest.skipUnless(sys.platform == 'linux', 'Linux bundled helper')
class DelegationBundledTest(DelegationCase):
    def setUp(self):
        super().setUp()
        self.info = json.loads(subprocess.check_output([str(self.client), '__cj_bundled_bwrap_info'], text=True))
        if not self.info['available']:
            self.skipTest('This source build did not prepare an embedded helper')

    def reject_system(self):
        fake = self.bin / 'bwrap'
        fake.write_text(f'#!{sys.executable}\nraise SystemExit(1)\n')
        fake.chmod(0o755)

    def test_bundled_helper_is_used_when_system_backend_fails(self):
        self.reject_system()
        repo = self.repository()
        identifier = self.start(target='codex', cwd=repo)
        snapshot = self.settled(identifier)
        self.assertEqual(snapshot['status'], 'completed', json.dumps(snapshot))
        self.assertEqual(snapshot['sandbox']['source'], 'bundled')
        self.assertTrue(snapshot['sandbox']['enforced'])
        helpers = list((self.root / 'state/codejournal/helpers').glob('*/bwrap'))
        self.assertEqual(len(helpers), 1)
        self.assertEqual(hashlib.sha256(helpers[0].read_bytes()).hexdigest(), self.info['sha256'])
        import struct
        data = helpers[0].read_bytes()
        offset = struct.unpack_from('<Q', data, 32)[0]
        stride, count = struct.unpack_from('<HH', data, 54)
        self.assertTrue(all(struct.unpack_from('<I', data, offset+i*stride)[0] != 3 for i in range(count)))
        self.assertTrue((helpers[0].parent / 'COPYING.bubblewrap').is_file())
        for name in ('COPYING.libcap', 'COPYING.musl', 'COPYING.gcc-runtime', 'COPYING.gcc-gpl'):
            self.assertTrue((helpers[0].parent / name).is_file())
        self.assertEqual(helpers[0].parent.stat().st_mode & 0o777, 0o700)

    def test_corrupt_cached_helper_is_replaced_with_verified_bytes(self):
        self.reject_system()
        self.settled(self.start())
        helper = next((self.root / 'state/codejournal/helpers').glob('*/bwrap'))
        helper.chmod(0o700)
        helper.write_bytes(b'damaged executable')
        result = self.settled(self.start())
        self.assertEqual(result['status'], 'completed')
        self.assertEqual(result['sandbox']['source'], 'bundled')
        self.assertEqual(hashlib.sha256(helper.read_bytes()).hexdigest(), self.info['sha256'])

    def test_single_binary_runs_without_any_system_bwrap(self):
        outer = shutil.which('bwrap')
        self.assertIsNotNone(outer)
        result = subprocess.run([outer, '--unshare-user', '--unshare-pid', '--die-with-parent',
            '--ro-bind', '/', '/', '--dev', '/dev', '--proc', '/proc', '--tmpfs', '/usr/bin',
            '--ro-bind', str(Path(sys.executable).resolve()), sys.executable,
            '--bind', str(self.root), str(self.root), '--', str(self.client),
            'delegation', 'run', 'codex', '--prompt', 'Inspect'],
            env=self.env, cwd=self.root, text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Synchronous fake provider response', result.stdout)
        self.assertNotIn('without filesystem sandbox', result.stderr)
        self.assertTrue(list((self.root / 'state/codejournal/helpers').glob('*/bwrap')))

    def test_real_namespace_denial_continues_without_any_backend(self):
        outer = shutil.which('bwrap')
        self.assertIsNotNone(outer)
        result = subprocess.run([outer, '--unshare-user', '--disable-userns',
            '--die-with-parent', '--ro-bind', '/', '/', '--dev', '/dev', '--proc', '/proc',
            '--bind', str(self.root), str(self.root), '--', str(self.client),
            'delegation', 'run', 'codex', '--prompt', 'Inspect'],
            env=self.env, cwd=self.root, text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Synchronous fake provider response', result.stdout)
        self.assertIn('without filesystem sandbox', result.stderr)

    def test_symlink_cache_entry_continues_without_touching_its_target(self):
        self.reject_system()
        self.settled(self.start())
        helper = next((self.root / 'state/codejournal/helpers').glob('*/bwrap'))
        outside = self.root / 'untouched-file'
        outside.write_text('preserve')
        helper.unlink()
        helper.symlink_to(outside)
        result = self.settled(self.start())
        self.assertEqual(result['status'], 'completed')
        self.assertFalse(result['sandbox']['enforced'])
        self.assertIn('not a regular file', result['sandbox']['reason'])
        self.assertEqual(outside.read_text(), 'preserve')
