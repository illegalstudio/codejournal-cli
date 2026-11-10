import datetime
import json
from delegation_support import DelegationCase


class DelegationStreamMarkersTest(DelegationCase):
    def test_summaries_skip_standalone_stream_markers_and_date_the_latest_event(self):
        tool = {'type': 'assistant', 'message': {'content': [
            {'type': 'tool_use', 'name': 'run_terminal_cmd', 'input': {'command': 'cargo test --workspace'}}]}}
        marker = {'type': 'stream_event', 'event': {'type': 'content_block_delta', 'index': 0,
                  'delta': {'type': 'input_json_delta', 'partial_json': '{"comm'}}}
        stop = {'type': 'stream_event', 'event': {'type': 'content_block_stop', 'index': 0}}
        emit = '\n'.join(f'print({json.dumps(event)!r}, flush=True)' for event in [tool] + [marker] * 40 + [stop])
        fake = self.bin / 'grok'
        fake.write_text(fake.read_text().replace('publish("ready", "Protocol loaded")',
                                               'publish("ready", "Protocol loaded")\n' + emit))
        identifier = self.start(target='grok')
        self.settled(identifier)
        stream = self.call('output', identifier, '--lines', '12', '--max-chars', '2000')['streams']['stdout']
        self.assertEqual(stream['format'], 'summary')
        self.assertNotIn('type=stream_event', stream['text'])
        self.assertNotIn('content_block_stop', stream['text'])
        self.assertIn('run_terminal_cmd', stream['text'])
        self.assertIn('cargo test --workspace', stream['text'])
        latest = stream['latest_event']
        self.assertIn('run_terminal_cmd', latest['text'])
        self.assertIn('cargo test --workspace', latest['text'])
        stamp = datetime.datetime.strptime(latest['at'], '%Y-%m-%dT%H:%M:%SZ').replace(tzinfo=datetime.timezone.utc)
        self.assertLess(abs((datetime.datetime.now(datetime.timezone.utc) - stamp).total_seconds()), 120)
        raw = self.call('output', identifier, '--raw', '--max-chars', '4000')['streams']['stdout']
        self.assertIn('input_json_delta', raw['text'])
        self.assertIsNone(raw['latest_event'])
