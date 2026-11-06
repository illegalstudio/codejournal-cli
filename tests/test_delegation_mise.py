import json
import os
import sys
from delegation_support import DelegationCase
from delegation_fake import FAKE


class DelegationMiseTest(DelegationCase):
    def setUp(self):
        super().setUp()
        self.wrappers = self.root / 'mise/command-wrappers/bin'
        self.wrappers.mkdir(parents=True)
        self.installed = self.root / 'installed/provider/bin'
        self.installed.mkdir(parents=True)
        runtime = self.root / 'installed/runtime/bin'
        runtime.mkdir(parents=True)
        (runtime / 'node').symlink_to(sys.executable)
        self.calls = self.root / 'mise-queries'
        self.mapping = {name: str(self.installed / name) for name in ('grok', 'kimi')}
        self.mapping['node'] = str(runtime / 'node')
        for name in ('grok', 'kimi'):
            program = self.installed / name
            program.write_text('#!/usr/bin/env node\n' + FAKE)
            program.chmod(0o755)
        self.manager = self.bin / 'mise'
        self.write_manager()
        for name in ('grok', 'kimi', 'node'):
            (self.wrappers / name).symlink_to(self.manager)
        self.env['PATH'] = os.pathsep.join([str(self.wrappers), str(self.bin)])

    def write_manager(self):
        body = f'''#!{sys.executable}
import json, os, sys
from pathlib import Path
assert os.environ.get('CJ_DELEGATION_SANDBOX') != '1'
assert os.environ['MISE_NOT_FOUND_AUTO_INSTALL'] == 'false'
assert os.environ['MISE_NOT_FOUND_SYSTEM_FALLBACK'] == 'false'
assert os.environ['MISE_NO_ENV'] == '1'
assert sys.argv[1] == 'which'
name = sys.argv[2]
path = Path({str(self.calls)!r})
path.write_text((path.read_text() if path.exists() else '') + name + '\\n')
mapping = {self.mapping!r}
if name not in mapping: raise SystemExit(2)
print(mapping[name])
'''
        self.manager.write_text(body)
        self.manager.chmod(0o755)

    def test_installed_grok_and_kimi_bypass_wrappers_with_the_selected_runtime(self):
        for target in ('grok', 'kimi'):
            with self.subTest(target=target):
                snapshot = self.settled(self.start(target=target))
                self.assertEqual(snapshot['status'], 'completed', snapshot)
                self.assertEqual(snapshot['session']['mode'], 'read-only')
                if sys.platform == 'linux':
                    self.assertTrue(snapshot['sandbox']['enforced'])
        self.assertEqual(self.calls.read_text().splitlines(), ['grok', 'node', 'kimi', 'node'])

    def test_synchronous_launch_resolves_without_changing_provider_output(self):
        result = self.invoke('run', 'grok', '--prompt', 'Inspect only')
        self.assertIn('Synchronous fake provider response', result.stdout)

    def test_missing_installed_provider_is_actionable_and_does_not_install(self):
        del self.mapping['grok']
        self.write_manager()
        result = self.invoke('run', 'grok', '--prompt', 'Inspect only', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('mise which grok', result.stderr)
        self.assertIn('installed executable', result.stderr)
        self.assertEqual(self.calls.read_text(), 'grok\n')

    def test_dry_run_does_not_execute_mise(self):
        self.call('start', 'grok', '--prompt', 'Inspect only', '--dry-run')
        self.assertFalse(self.calls.exists())
