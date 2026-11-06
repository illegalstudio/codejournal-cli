import json
from delegation_support import DelegationCase


class DelegationNestedOutputTest(DelegationCase):
    def test_legacy_escaped_grok_events_and_messages_keep_text_and_tool_commands(self):
        legacy = {'type': 'text', 'data': json.dumps({'type': 'tool_call', 'toolName': 'run_terminal_cmd',
            'rawInput': {'command': 'git diff --stat', 'bytes': [65] * 20},
            'content': [{'type': 'text', 'text': 'Inspecting changed modules'}]})}
        message = {'type': 'assistant', 'message': {'content': [
            {'type': 'thinking', 'thinking': 'SYNTHETIC_PRIVATE_REASONING'},
            {'type': 'tool_use', 'name': 'read_file', 'input': {'path': 'src/example.rs'}},
            {'type': 'text', 'text': 'Checking a regression'}]}}
        delta = {'type': 'stream_event', 'event': {'type': 'content_block_delta',
                 'delta': {'type': 'text_delta', 'text': 'Partial review progress'}}}
        noise = {'type': 'thought', 'data': 'SYNTHETIC_PRIVATE_REASONING'}
        fake = self.bin / 'grok'
        emit = '\n'.join(f'print({json.dumps(event)!r}, flush=True)' for event in (legacy, message, delta, noise))
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")',
                                               'publish("ready", "Protocol loaded")\n' + emit))
        identifier = self.start(target='grok')
        self.settled(identifier)
        output = self.call('output', identifier, '--lines', '12', '--max-chars', '2000')['streams']['stdout']['text']
        for expected in ('run_terminal_cmd', 'git diff --stat', 'Inspecting changed modules',
                         'src/example.rs', 'Checking a regression', 'Partial review progress'):
            self.assertIn(expected, output)
        self.assertNotIn('SYNTHETIC_PRIVATE_REASONING', output)
        self.assertNotIn('65, 65, 65', output)
        self.assertLessEqual(len(output), 2000)

    def test_large_encoded_payloads_do_not_fill_summaries_with_byte_arrays(self):
        large = {'type': 'text', 'data': json.dumps({'rawOutput': {'stdout': [65] * 20000}})}
        fake = self.bin / 'grok'
        emit = f'print({json.dumps(large)!r}, flush=True)\nprint("LATEST_NESTED_PROGRESS", flush=True)'
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")',
                                               'publish("ready", "Protocol loaded")\n' + emit))
        identifier = self.start(target='grok')
        self.settled(identifier)
        output = self.call('output', identifier, '--max-chars', '1000')['streams']['stdout']['text']
        self.assertIn('LATEST_NESTED_PROGRESS', output)
        self.assertNotIn('65, 65, 65', output)
