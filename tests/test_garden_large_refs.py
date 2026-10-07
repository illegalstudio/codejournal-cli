import hashlib
import json
import subprocess
from unittest.mock import patch

import test_garden_review as modern
from feedback_audit_support import AuditCase, Handler, RESOURCE


class GardenLargeReferenceTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.files = self.repo / 'src'
        self.files.mkdir()
        self.enterContext(patch.object(Handler, 'do_GET', lambda handler: self.read(handler)))
        self.enterContext(patch.object(Handler, 'do_POST', modern.write))
        self.git('config', 'user.email', 'test@example.test')
        self.git('config', 'user.name', 'Test')

    def read(self, handler):
        if '/garden?' in handler.path:
            handler.reply(200, {'entries': [{'id': RESOURCE, 'refs': [{'kind': 'path', 'value': 'src'}]}],
                                'docs': [], 'next_page': None})
        else:
            modern.read(handler)

    def git(self, *args, stdin=None):
        result = subprocess.run(['git', '-c', 'commit.gpgsign=false', *args], cwd=self.repo,
                                env=self.env, input=stdin, text=True, capture_output=True, check=True)
        return result.stdout.strip()

    def snapshot(self, dry_run=True):
        Handler.calls.clear()
        result = self.cli('--json', '--project', 'fixture', 'garden', *(['--dry-run'] if dry_run else []))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(json.loads(result.stdout)['findings']), 5)
        return next(body['entries'][0]['code_hash'] for _, path, body in Handler.calls
                    if path.endswith(('/scan', '/preview')) and body.get('entries'))

    def expected(self, names):
        fingerprint = hashlib.sha256(b'src\0present')
        for name in sorted(names, key=lambda item: item.encode()):
            fingerprint.update(('src/' + name).encode())
            fingerprint.update((self.files / name).read_bytes())
            fingerprint.update(b'\0')
        return fingerprint.hexdigest()

    def test_more_than_1000_files_scan_completely_and_keep_existing_fingerprints(self):
        names = [f'file-{i:04}.rs' for i in range(1005)]
        for name in names:
            (self.files / name).write_text(name + '\n')
        self.git('add', 'src')
        self.git('commit', '-m', 'Large source directory')
        (self.repo / '.gitignore').write_text('src/ignored.rs\n')
        (self.files / 'ignored.rs').write_text('Ignored contents\n')
        (self.repo / 'unrelated.rs').write_text('Outside reference\n')
        reviewed = self.snapshot(dry_run=False)
        self.assertEqual(reviewed, self.expected(names))
        original = self.repo
        try:
            self.repo = self.files
            self.assertEqual(self.snapshot(), reviewed)
        finally:
            self.repo = original
        new_names = ['aaa.rs', 'file-0000-new.rs', 'new\nline.rs', 'zzz.rs']
        for name in new_names:
            (self.files / name).write_text('New source\n')
        names.extend(new_names)
        changed = self.snapshot()
        self.assertNotEqual(changed, reviewed)
        self.assertEqual(changed, self.expected(names))
        self.git('add', 'src')
        self.git('commit', '-m', 'Commit reviewed new files')
        self.assertEqual(self.snapshot(), changed)
        (self.files / names[0]).unlink()
        names.pop(0)
        deleted = self.snapshot()
        self.assertEqual(deleted, self.expected(names))
        self.git('add', 'src')
        self.git('commit', '-m', 'Commit reviewed deletion')
        self.assertEqual(self.snapshot(), deleted)
        (self.files / 'ignored.rs').write_text('Changed ignored contents\n')
        self.assertEqual(self.snapshot(), deleted)

    def test_unmerged_index_duplicates_are_hashed_once(self):
        (self.files / 'conflicted.rs').write_text('Uncommitted resolution\n')
        blob = self.git('hash-object', '-w', '--stdin', stdin='Synthetic blob\n')
        rows = ''.join(f'100644 {blob} {stage}\tsrc/conflicted.rs\n' for stage in [1, 2, 3])
        self.git('update-index', '--index-info', stdin=rows)
        self.assertEqual(self.snapshot(), self.expected(['conflicted.rs']))
