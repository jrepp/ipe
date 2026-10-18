#!/usr/bin/env python3
"""Independent client boundary tests; no live provider or credential required."""
import http.client
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
CLIENT = ROOT / 'scripts' / 'integration-toolkit.py'
spec = importlib.util.spec_from_file_location('integration_toolkit', CLIENT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
TOKEN = 'a' * 40
SENSITIVE = 'provider-secret internal.private.example'

class ClientBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.env = patch.dict(os.environ, {'IPE_INTEGRATION_TOKEN': TOKEN})
        self.env.start()
        self.servers = []

    def tearDown(self):
        for server in self.servers:
            server.shutdown()
            server.server_close()
        self.env.stop()

    def server(self, mode):
        requests = []
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args): pass
            def do_GET(self):
                requests.append((self.path, self.headers.get('Authorization')))
                if mode == 'redirect':
                    self.send_response(302)
                    self.send_header('Location', '/credential-exfiltration')
                    self.end_headers()
                else:
                    self.send_response(200)
                    self.send_header('Content-Length', '999')
                    self.end_headers()
                    self.wfile.write(SENSITIVE.encode())
                    self.close_connection = True
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.servers.append(server)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        return f'http://127.0.0.1:{server.server_port}', requests

    def test_truncated_http_sanitized(self):
        url, _requests = self.server('truncated')
        result = subprocess.run([sys.executable, str(CLIENT), '--url', url, 'capabilities'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn(json.loads(result.stdout)['error']['code'], ('workload_unavailable', 'invalid_workload_response'))
        self.assertEqual(result.stderr, '')
        self.assertNotIn(SENSITIVE, result.stdout)

    def test_http_exception_sanitized(self):
        client = module.Client('http://127.0.0.1:1', 'IPE_INTEGRATION_TOKEN')
        with patch.object(client.opener, 'open', side_effect=http.client.IncompleteRead(SENSITIVE.encode(), 999)):
            with self.assertRaises(module.ToolkitError) as caught:
                client.call('capabilities')
        self.assertEqual(caught.exception.code, 'workload_unavailable')

    def test_redirect_not_followed(self):
        url, requests = self.server('redirect')
        with self.assertRaises(module.ToolkitError) as caught:
            module.Client(url, 'IPE_INTEGRATION_TOKEN').call('capabilities')
        self.assertEqual(caught.exception.code, 'workload_request_failed')
        self.assertEqual(requests, [('/v1/capabilities', 'Bearer ' + TOKEN)])

    def test_invalid_endpoints(self):
        for url in ('http://192.0.2.1', 'http://localhost', 'file:///tmp/token', 'https://user:secret@example.com', 'https://example.com/path', 'https://example.com/?query=secret', 'https://example.com/#secret'):
            with self.subTest(url=url), self.assertRaises(module.ToolkitError) as caught:
                module.Client(url, 'IPE_INTEGRATION_TOKEN')
            self.assertEqual(caught.exception.code, 'invalid_workload_endpoint')

    def test_mcp_invalid_inputs_are_local_safe_errors(self):
        lines = ['{bad json']
        messages = [
            {'jsonrpc':'2.0','id':1,'method':'tools/list'},
            {'jsonrpc':'2.0','id':2,'method':'initialize','params':{'protocolVersion':'2025-11-25'}},
            {'jsonrpc':'2.0','method':'notifications/initialized'},
            {'jsonrpc':'2.0','id':3,'method':'tools/list'},
            {'jsonrpc':'2.0','id':4,'method':'tools/call','params':{'name':'arbitrary_http','arguments':{'url':SENSITIVE}}},
            {'jsonrpc':'2.0','id':5,'method':'tools/call','params':{'name':'integration_discover','arguments':{'connection_ref':'conn_test','url':SENSITIVE}}},
            {'jsonrpc':'2.0','id':6,'method':'tools/call','params':{'name':'integration_discover','arguments':{'connection_ref':'../../secrets'}}},
        ]
        lines += [json.dumps(value) for value in messages]
        result = subprocess.run([sys.executable, str(CLIENT), '--url', 'http://127.0.0.1:1', 'mcp'], input='\n'.join(lines)+'\n', capture_output=True, text=True)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, '')
        self.assertNotIn(SENSITIVE, result.stdout)
        replies = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual(replies[0]['error']['code'], -32700)
        by_id = {reply['id']:reply for reply in replies}
        self.assertEqual(by_id[1]['error']['code'], -32000)
        self.assertEqual(len(by_id[3]['result']['tools']), 3)
        for request_id in (4,5,6): self.assertEqual(by_id[request_id]['error']['code'], -32602)

if __name__ == '__main__':
    unittest.main()
