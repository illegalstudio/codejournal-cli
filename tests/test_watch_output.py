import json
import os
import pathlib
import subprocess
import time
from unittest.mock import patch
from watch_support import WatchCase, Handler, ROOT


class WatchOutputTest(WatchCase):
    def start(self):
        self.env['CODE_JOURNAL_AUTO_SYNC'] = 'off'
        result = json.loads(self.cli('--json', 'watch', 'start', '--title', 'Output', '--',
                                    'sh', '-c', 'echo first; echo second >&2; sleep 30'))
        ident = result['watch']['id']
        path = pathlib.Path(self.temp.name) / 'state/codejournal/watches' / f'{ident}.out'
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline and (not path.exists() or 'second' not in path.read_text()):
            time.sleep(0.02)
        return ident, path

    def output(self, ident, *flags):
        return json.loads(self.cli('--json', *flags, 'watch', 'output', ident))

    def test_running_output_is_scoped_bounded_and_available_offline_with_explicit_metadata(self):
        ident, path = self.start()
        result = self.output(ident[:8])
        self.assertEqual(result['output']['tail'], 'first\nsecond')
        self.assertEqual(result['output']['source'], 'local')
        self.assertTrue(result['runner_verified'])
        self.assertGreater(result['runner_pid'], 0)
        self.assertIsNotNone(result['output']['last_output_at'])
        self.assertFalse(result['cached'])
        Handler.paths.clear()
        result = self.output(ident[:8], '--offline')
        self.assertTrue(result['cached'])
        self.assertEqual(result['output']['source'], 'local')
        self.assertEqual(Handler.paths, [])
        path.write_text('x' * 40000 + '\nthird\nfourth\n')
        result = json.loads(self.cli('--json', 'watch', 'output', ident, '--lines', '2'))
        self.assertEqual(result['output']['tail'], 'third\nfourth')
        self.assertTrue(result['output']['truncated'])
        self.assertEqual(result['output']['bytes'], path.stat().st_size)
        path.write_text('é' * 30000)
        self.assertLessEqual(len(self.output(ident)['output']['tail']), 10000)
        self.assertIn('does not prove command progress', self.cli('watch', 'output', ident))

    def test_scope_legacy_pid_and_reused_pid_cannot_disclose_local_output(self):
        ident, path = self.start()
        pid_path = path.with_suffix('.pid')
        original = json.loads(pid_path.read_text())
        try:
            for record in [dict(original, scope='another-scope'), dict(original, pid=os.getpid()), original['pid']]:
                pid_path.write_text(json.dumps(record))
                result = self.output(ident)
                self.assertFalse(result['runner_verified'])
                self.assertEqual(result['output']['source'], 'unavailable')
                self.assertNotIn('second', json.dumps(result))
        finally:
            pid_path.write_text(json.dumps(original))
        self.env['CJ_TOKEN'] = 'different-token'
        self.assertEqual(self.output(ident)['output']['source'], 'unavailable')
        self.env['CJ_TOKEN'] = 'test-token'

    def test_symlinks_and_remote_access_denial_never_expose_output(self):
        ident, path = self.start()
        target = pathlib.Path(self.temp.name) / 'unrelated.txt'
        target.write_text('PRIVATE LOCAL CONTENT')
        path.unlink()
        path.symlink_to(target)
        self.assertEqual(self.output(ident)['output']['source'], 'unavailable')
        with patch.object(Handler, 'do_GET', lambda h: h.respond({'message': 'Token revoked'}, 401)):
            result = subprocess.run([self.binary, '--json', 'watch', 'output', ident], env=self.env,
                                    cwd=ROOT, capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('PRIVATE LOCAL CONTENT', result.stdout + result.stderr)

    def test_completed_watch_uses_server_tail_and_rejects_invalid_bounds(self):
        result = json.loads(self.cli('--json', 'watch', 'start', '--title', 'Finished', '--', 'echo', 'done'))
        ident = result['watch']['id']
        self.wait(ident, 'finished')
        snapshot = self.output(ident)
        self.assertEqual(snapshot['output']['source'], 'server')
        self.assertEqual(snapshot['output']['tail'], 'done')
        self.assertFalse(snapshot['runner_verified'])
        self.assertTrue(self.output(ident, '--offline')['cached'])
        Handler.paths.clear()
        for args in [(ident, '--lines', '0'), (ident, '--lines', '201'), ('../../private',)]:
            result = subprocess.run([self.binary, 'watch', 'output', *args], env=self.env,
                                    cwd=ROOT, capture_output=True, text=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(Handler.paths, [])
