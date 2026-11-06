import json
from feedback_audit_support import AuditCase, RESOURCE


class EntryAddIdTest(AuditCase):
    def test_confirmed_entry_has_scriptable_id_and_preserves_existing_envelope(self):
        result = self.cli('--project', 'fixture', '--json', 'add', '--kind', 'discovery',
                          '--title', 'Scriptable entry', '--body', 'Synthetic content')
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(result.stdout)
        self.assertEqual(data['id'], RESOURCE)
        self.assertEqual(data['entry']['id'], RESOURCE)
        self.assertEqual(data['resource_id'], RESOURCE)
        self.assertFalse(data['queued'])

    def test_queued_entry_does_not_claim_a_confirmed_resource_id(self):
        self.test_confirmed_entry_has_scriptable_id_and_preserves_existing_envelope()
        result = self.cli('--project', 'fixture', '--json', '--offline', 'add', '--kind', 'discovery',
                          '--title', 'Pending entry', '--body', 'Synthetic content')
        data = json.loads(result.stdout)
        self.assertTrue(data['queued'])
        self.assertNotIn('entry', data)
        self.assertIsNone(data['resource_id'])
        self.assertEqual(data['id'], data['request_id'])
