"""Capture the real Claude Code ToolSearch lifecycle against a loopback fixture."""
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]
MODEL = os.environ.get('CONNECT_FIXTURE_MODEL','claude-sonnet-4-6')
SCENARIO = os.environ.get('CONNECT_FIXTURE_SCENARIO', 'default')
FIXTURE = ROOT / 'test-results/claude-runtime' / (MODEL + '-' + SCENARIO)
NAME = 'mcp__fixture__lookup_marker'

if '--mcp' in sys.argv:
    for line in sys.stdin:
        q = json.loads(line)
        if 'id' not in q:
            continue
        method = q.get('method')
        if method == 'initialize':
            r = {'protocolVersion': q['params']['protocolVersion'], 'capabilities': {'tools': {}}, 'serverInfo': {'name': 'fixture', 'version': '1'}}
        elif method == 'tools/list':
            r = {'tools': [{'name': 'lookup_marker', 'description': 'Read the synthetic verification marker. No external data or actions.', 'inputSchema': {'type': 'object', 'properties': {}, 'additionalProperties': False}, 'annotations': {'readOnlyHint': True}}]}
            r['tools'] += [{'name': 'unused_' + str(i), 'description': 'Unused read-only development fixture. ' * 20, 'inputSchema': {'type':'object','properties':{'query':{'type':'string'}}}} for i in range(99)]
        else:
            r = {'content': [{'type': 'text', 'text': 'CONNECT_MARKER_7e219'}]}
        print(json.dumps({'jsonrpc': '2.0', 'id': q['id'], 'result': r}), flush=True)
    sys.exit(0)

FIXTURE.mkdir(parents=True, exist_ok=True)
requests = []

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if not self.path.startswith('/v1/messages') or 'count_tokens' in self.path:
            raw = json.dumps({'input_tokens': 10}).encode()
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.end_headers(); self.wfile.write(raw)
            return
        assert self.headers.get('Authorization') == 'Bearer sk-local-fixture'
        assert not self.headers.get('X-Api-Key'), 'Stale inherited API key reached gateway'
        assert body['model'] == MODEL, body['model']
        requests.append(body)
        index = len(requests)
        (FIXTURE / f'request-{index}.json').write_text(json.dumps(body, indent=2))
        if index == 1:
            block = {'type': 'tool_use', 'id': 'toolu_search_fixture', 'name': 'ToolSearch', 'input': {'query': 'select:' + NAME, 'max_results': 1}}
        elif index == 2:
            block = {'type': 'tool_use', 'id': 'toolu_marker_fixture', 'name': NAME, 'input': {}}
        else:
            block = {'type': 'text', 'text': 'CONNECT_MARKER_7e219'}
        stop = 'end_turn' if block['type'] == 'text' else 'tool_use'
        message = {'id': f'msg_fixture_{index}', 'type': 'message', 'role': 'assistant', 'model': body['model'], 'content': [], 'stop_reason': None, 'stop_sequence': None, 'usage': {'input_tokens': 10, 'output_tokens': 10}}
        events = [('message_start', {'type': 'message_start', 'message': message})]
        if block['type'] == 'tool_use':
            events += [('content_block_start', {'type': 'content_block_start', 'index': 0, 'content_block': {**block, 'input': {}}}), ('content_block_delta', {'type': 'content_block_delta', 'index': 0, 'delta': {'type': 'input_json_delta', 'partial_json': json.dumps(block['input'])}})]
        else:
            events += [('content_block_start', {'type': 'content_block_start', 'index': 0, 'content_block': {'type': 'text', 'text': ''}}), ('content_block_delta', {'type': 'content_block_delta', 'index': 0, 'delta': {'type': 'text_delta', 'text': block['text']}})]
        events += [('content_block_stop', {'type': 'content_block_stop', 'index': 0}), ('message_delta', {'type': 'message_delta', 'delta': {'stop_reason': stop, 'stop_sequence': None}, 'usage': {'output_tokens': 10}}), ('message_stop', {'type': 'message_stop'})]
        raw = ''.join(f'event: {event}\ndata: {json.dumps(data)}\n\n' for event, data in events).encode()
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.send_header('Content-Length', str(len(raw))); self.end_headers(); self.wfile.write(raw)

server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
env = {k: os.environ[k] for k in ['PATH', 'TMPDIR', 'LANG', 'LC_ALL', 'SystemRoot', 'SYSTEMROOT', 'WINDIR', 'TEMP', 'TMP', 'COMSPEC', 'PATHEXT'] if k in os.environ}
fixture_binary = ROOT / ('src-tauri/target/debug/examples/claude_fixture.exe' if os.name == 'nt' else 'src-tauri/target/debug/examples/claude_fixture')
fixture_args = [str(fixture_binary), str(FIXTURE/'home'), f'http://127.0.0.1:{server.server_port}', MODEL, 'supported']
if SCENARIO == 'role':
    fixture_args[3] = 'claude-opus-5-5'
    fixture_args.append(json.dumps(['claude-opus-5-5', MODEL]))
config_dir = subprocess.check_output(fixture_args,text=True).strip()
env.update({'CLAUDE_CONFIG_DIR': config_dir, 'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC': '1', 'MCP_CONNECTION_NONBLOCKING': '0', 'CLAUDE_CODE_MAX_OUTPUT_TOKENS': '512'})
mcp = {'mcpServers': {'fixture': {'command': sys.executable, 'args': [str(Path(__file__)), '--mcp'], 'alwaysLoad': False}}}
if SCENARIO == 'inherited':
    env.update({'ANTHROPIC_API_KEY':'stale-key', 'ANTHROPIC_MODEL':'stale-model', 'CLAUDE_CODE_USE_VERTEX':'1', 'CLAUDE_CODE_USE_BEDROCK':'1', 'CLAUDE_CODE_OAUTH_TOKEN':'stale-oauth', 'ANTHROPIC_CUSTOM_HEADERS':'X-Old: stale'})
argv = [shutil.which('claude') or 'claude', '--setting-sources', 'user', '--settings', '{"disableAllHooks":true}', '--system-prompt', 'You are a synthetic compatibility tester. Follow the user and only use the provided tools.', '--strict-mcp-config', '--mcp-config', json.dumps(mcp), '--tools', 'ToolSearch', '--allowedTools', NAME, '--disable-slash-commands', '--output-format', 'json', '--max-turns', '4', '-p', 'Use ToolSearch to find the fixture lookup_marker tool. Call that tool and reply with the exact marker it returns.']
if SCENARIO == 'role':
    argv[1:1] = ['--model', 'haiku']
try:
    p = subprocess.run(argv, cwd=FIXTURE, env=env, text=True, capture_output=True, timeout=60)
    (FIXTURE / 'stdout.txt').write_text(p.stdout)
    (FIXTURE / 'stderr.txt').write_text(p.stderr)
    result = {'exit': p.returncode, 'request_count': len(requests), 'tools': [[t.get('name') for t in r.get('tools', [])] for r in requests], 'initial_mcp_schemas': sum(t.get('name','').startswith('mcp__') for t in requests[0].get('tools',[])) if requests else -1, 'has_reference': any('tool_reference' in json.dumps(r) for r in requests), 'has_marker_result': len(requests) >= 3 and 'CONNECT_MARKER_7e219' in json.dumps(requests[2]['messages'])}
    (FIXTURE / 'capture.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result))
    assert p.returncode == 0 and len(requests) == 3 and result['initial_mcp_schemas'] == 0 and result['has_reference'] and result['has_marker_result']
finally:
    server.shutdown(); server.server_close()
