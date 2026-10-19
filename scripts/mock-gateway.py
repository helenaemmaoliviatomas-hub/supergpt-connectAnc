"""Local UI smoke-test gateway. Only accepts a fake fixture key; never logs headers."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from urllib.parse import urlsplit

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        path = urlsplit(self.path).path
        if path not in ('/v1/models', '/v1/sub2api/connect'):
            self.send_error(404)
            return
        allowed = self.headers.get('Authorization') == 'Bearer sk-local-fixture-only'
        rows = [
            {'id': 'gpt-6.1-sol', 'name': 'GPT · Codex 测试模型', 'input_modalities': ['text','image']},
            {'id': 'claude-sonnet-4-6', 'name': 'Claude · Sonnet 测试模型'},
            {'id': 'claude-opus-5-5', 'name': 'Claude · Opus 测试模型'},
            {'id': 'claude-haiku-4-5', 'name': 'Claude · Haiku 测试模型'},
            {'id': 'custom-model-test', 'context_window': 131072},
            {'id': 'gpt-image-2'},
        ]
        payload = {'data': rows}
        if path == '/v1/sub2api/connect':
            payload = {'object': 'supergpt.connect_capabilities', 'schema_version': 1, 'models': {
                row['id']: {'messages': {'status': 'supported' if row['id'].startswith('claude-') else 'unsupported'},
                            'claude_tool_search': {'status': 'supported' if row['id'].startswith('claude-') else 'unknown'}} for row in rows}}
        if not allowed:
            payload = {'error': 'unauthorized'}
        body = json.dumps(payload).encode()
        self.send_response(200 if allowed else 401)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass

if __name__ == '__main__':
    print('Local fixture gateway: http://127.0.0.1:18473', flush=True)
    HTTPServer(('127.0.0.1', 18473), Handler).serve_forever()
