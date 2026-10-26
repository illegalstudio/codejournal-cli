import hashlib
import json
import subprocess
import urllib.parse
import uuid
from unittest.mock import patch

import test_garden_review as modern
from feedback_audit_support import AuditCase, Handler


class GardenScanBatchTest(AuditCase):
    def setUp(self):
        super().setUp()
        self.pages = []
        self.enterContext(patch.object(Handler, 'do_GET', lambda handler: self.read(handler)))
        self.enterContext(patch.object(Handler, 'do_POST', modern.write))
        (self.repo / 'README.md').write_text('Synthetic source\n')

    def rows(self, count, offset=1, refs=None):
        return [{'id': str(uuid.UUID(int=offset + i)), 'content_hash': 'a' * 64,
                 'refs': refs if refs is not None else [{'kind': 'path', 'value': 'README.md'}]}
                for i in range(count)]

    def read(self, handler):
        if '/garden?' not in handler.path:
            return modern.read(handler)
        Handler.calls.append(('GET', handler.path, None))
        query = urllib.parse.parse_qs(urllib.parse.urlparse(handler.path).query)
        page = int(query['page'][0])
        entries, docs = self.pages[page - 1]
        handler.reply(200, {'entries': entries, 'docs': docs,
                            'next_page': page + 1 if page < len(self.pages) else None})

    def scan(self, *args):
        result = self.cli('--json', '--project', 'fixture', 'garden', *args)
        self.assertEqual(result.returncode, 0, result.stderr)
        requests = [body for method, path, body in Handler.calls
                    if method == 'POST' and path.endswith(('/scan', '/preview'))]
        for body in requests:
            self.assertLessEqual(len(body.get('entries', [])), 250)
            self.assertLessEqual(len(body.get('docs', [])), 250)
            self.assertLessEqual(len(body.get('observations', [])), 1000)
            for row in body.get('observations', []):
                for key in ['branches', 'commits']:
                    self.assertLessEqual(len(row[key]), 30)
            self.assertLessEqual(len(json.dumps(body, separators=(',', ':')).encode()), 4 * 1024 * 1024)
        self.assertEqual([bool(body.get('complete')) for body in requests],
                         [False] * (len(requests) - 1) + [True])
        return requests, json.loads(result.stdout)

    def test_full_metadata_page_combines_entries_docs_and_completion_in_one_request(self):
        entries, docs = self.rows(250), self.rows(250, offset=1000)
        self.pages = [(entries, docs)]
        requests, data = self.scan()
        self.assertEqual(len(requests), 1)
        self.assertEqual(len(data['findings']), 5)
        self.assertEqual(data['counts']['pending'], 7)
        expected = hashlib.sha256(b'README.md\0presentSynthetic source\n\0').hexdigest()
        for key, rows in [('entries', entries), ('docs', docs)]:
            self.assertEqual([row['id'] for row in requests[0][key]], [row['id'] for row in rows])
            self.assertTrue(all(row['code_hash'] == expected and row['content_hash'] == 'a' * 64
                                for row in requests[0][key]))

    def test_multiple_pages_complete_only_after_all_observations_and_accumulate_preview(self):
        self.pages = [(self.rows(250), self.rows(10, offset=1000)), (self.rows(10, offset=300), [])]
        original = modern.write
        def write(handler):
            if handler.path.endswith('/preview'):
                body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
                Handler.calls.append(('POST', handler.path, body))
                handler.reply(200, {'findings': modern.FINDINGS[:2] if not body.get('complete') else modern.FINDINGS[2:],
                                    'reviewed': 1, 'deferred': 0, 'progress': modern.COUNTS})
            else:
                original(handler)
        with patch.object(Handler, 'do_POST', write):
            requests, data = self.scan('--dry-run', '--all')
        self.assertEqual(len(requests), 2)
        self.assertEqual(sum(len(body['entries']) for body in requests), 260)
        self.assertEqual(sum(len(body['docs']) for body in requests), 10)
        self.assertEqual(len(data['findings']), 7)
        self.assertEqual(data['counts']['reviewed'], 2)

    def test_empty_checkout_metadata_still_completes_report(self):
        self.pages = [([], [])]
        requests, _ = self.scan()
        self.assertEqual(requests, [{'complete': True}])

    def test_path_limit_splits_without_losing_or_duplicating_records(self):
        entries = self.rows(51, refs=[])
        for i, row in enumerate(entries):
            row['refs'] = [{'kind': 'path', 'value': f'gone/{i}/{j}.rs'} for j in range(20)]
        self.pages = [(entries, self.rows(2, offset=1000))]
        requests, _ = self.scan('--dry-run')
        self.assertGreater(len(requests), 1)
        self.assertEqual([row['id'] for body in requests for row in body['entries']],
                         [row['id'] for row in entries])
        self.assertEqual(sum(len(body['docs']) for body in requests), 2)
        self.assertEqual(sum(len(body['observations']) for body in requests), 1020)

    def test_shared_path_candidate_limit_splits_and_preserves_every_branch(self):
        def git(*args):
            subprocess.run(['git', '-c', 'commit.gpgsign=false', *args], cwd=self.repo, env=self.env,
                           check=True, capture_output=True)
        git('config', 'user.email', 'test@example.test')
        git('config', 'user.name', 'Test')
        git('add', 'README.md')
        git('commit', '-m', 'Synthetic source')
        entries = self.rows(31)
        branches = [f'review-{i}' for i in range(31)]
        for row, branch in zip(entries, branches):
            git('branch', branch)
            row['refs'].append({'kind': 'branch', 'value': branch})
        self.pages = [(entries, [])]
        requests, _ = self.scan('--dry-run')
        self.assertEqual(len(requests), 2)
        actual = [branch for body in requests for row in body['observations'] for branch in row['branches']]
        self.assertEqual(sorted(actual), sorted(branches))

    def test_failed_page_is_not_queued_and_never_sends_completion(self):
        self.pages = [(self.rows(1), []), (self.rows(1, offset=300), [])]
        def fail(handler):
            body = json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
            Handler.calls.append(('POST', handler.path, body))
            handler.reply(503, {'message': 'Synthetic scan failure'})
        with patch.object(Handler, 'do_POST', fail):
            result = self.cli('--json', '--project', 'fixture', 'garden', '--dry-run')
        self.assertNotEqual(result.returncode, 0)
        writes = [body for method, _, body in Handler.calls if method == 'POST']
        self.assertEqual(len(writes), 1)
        self.assertFalse(writes[0].get('complete'))
        self.assertEqual(json.loads(self.cli('--json', 'outbox', 'list').stdout)['requests'], [])
