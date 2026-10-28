#!/usr/bin/env python3
"""Install agent-authored JS in the endpoint and capture its run/evidence APIs."""
import argparse
import json
import os
from pathlib import Path
import secrets
import signal
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
REPOSITORIES = ['jrepp/ipe', 'actions/checkout', 'actions/setup-python', 'actions/setup-node', 'actions/upload-artifact', 'actions/upload-pages-artifact', 'actions/configure-pages', 'actions/deploy-pages']

def configuration(output, port):
    connection = {'id': 'github', 'origin': 'https://api.github.com/', 'headers': {'x-github-api-version': '2022-11-28'}}
    connection['git_credential_url'] = 'https://github.com'
    repo = {'kind': 'path', 'values': REPOSITORIES}
    root = {'kind': 'path', 'values': ['jrepp/ipe']}
    def op(name, path, parameters, decoder='json', fields=None, **kwargs):
        return dict(name=name, connection='github', path=path, parameters=parameters, decoder=decoder, fields=fields or [], **kwargs)
    operations = [
        op('resolve_revision', '/repos/{repository}/commits/{revision}', {'repository': repo, 'revision': {'kind': 'ref'}}, fields=['sha']),
        op('read_definition', '/repos/{repository}/contents/{path}', {'repository': repo, 'revision': {'kind': 'sha'}, 'path': {'kind': 'path'}}, decoder='yaml', accept='application/vnd.github.raw+json', query={'ref': '{revision}'}),
        op('read_source', '/repos/{repository}/contents/{path}', {'repository': repo, 'revision': {'kind': 'sha'}, 'path': {'kind': 'path'}}, decoder='text', accept='application/vnd.github.raw+json', query={'ref': '{revision}'}),
        op('list_tree', '/repos/{repository}/git/trees/{revision}', {'repository': repo, 'revision': {'kind': 'sha'}}, fields=['path', 'type'], pointer='/tree', max_records=128, query={'recursive': '1'}),
        op('list_runs', '/repos/{repository}/actions/workflows/{workflow}/runs', {'repository': root, 'revision': {'kind': 'sha'}, 'workflow': {'kind': 'identifier', 'values': ['docs-site.yml']}}, fields=['id', 'head_sha', 'name', 'event', 'path', 'status', 'conclusion', 'run_attempt', 'created_at', 'run_started_at', 'updated_at', 'html_url'], pointer='/workflow_runs', max_records=3, query={'head_sha': '{revision}', 'per_page': '3'}),
        op('list_jobs', '/repos/{repository}/actions/runs/{run_id}/attempts/{attempt}/jobs', {'repository': root, 'run_id': {'kind': 'integer'}, 'attempt': {'kind': 'integer'}}, fields=['id', 'name', 'status', 'conclusion', 'started_at', 'completed_at', 'steps', 'head_sha'], pointer='/jobs', max_records=6, query={'per_page': '6'}),
    ]
    return {'listen': f'127.0.0.1:{port}', 'api_token_env': 'IPE_HOST_OPERATOR_TOKEN', 'state_dir': str(output / 'state'), 'connections': [connection], 'operations': operations}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, default=ROOT / 'target/integration-host-demo')
    parser.add_argument('--port', type=int, default=8767)
    parser.add_argument('--serve', action='store_true')
    parser.add_argument('--prove-schedule', action='store_true')
    args = parser.parse_args()
    output = args.out.resolve()
    output.mkdir(parents=True, exist_ok=True)
    token = secrets.token_urlsafe(40)
    config = configuration(output, args.port)
    config_path = output / 'host-config.json'
    config_path.write_text(json.dumps(config, indent=2)+'\n')
    env_path = output / 'operator.env'
    fd = os.open(env_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, 'w') as f:
        f.write('export IPE_HOST_OPERATOR_TOKEN='+token+'\n')
    env = dict(os.environ, IPE_HOST_OPERATOR_TOKEN=token)
    process = subprocess.Popen([str(ROOT/'target/debug/ipe-integration'), str(config_path), '--host'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    base = f'http://127.0.0.1:{args.port}'
    def api(path, method='GET', data=None):
        request = urllib.request.Request(base+path, method=method, data=None if data is None else json.dumps(data).encode(), headers={'Authorization': 'Bearer '+token, 'Content-Type': 'application/json'})
        with urllib.request.urlopen(request, timeout=15) as response:
            return json.load(response)
    package = {'source': (ROOT/'testing/integration/hosted/github-actions.js').read_text(), 'operations': [o['name'] for o in config['operations']], 'input': {'repository': 'jrepp/ipe', 'revision': '8346e53d0b428b701349817df6624f0ad33a1639', 'workflow': '.github/workflows/docs-site.yml', 'allowed_repositories': REPOSITORIES}, 'interval_seconds': 2 if args.prove_schedule else 300, 'enabled': True, 'fact_ttl_seconds': 900, 'output_kinds': ['contract', 'fact', 'outcome']}
    try:
        for _ in range(100):
            if process.poll() is not None:
                raise RuntimeError('Host failed to start: '+process.stderr.read().decode()[:100])
            try:
                api('/v1/host/capabilities')
                break
            except (OSError, urllib.error.URLError):
                time.sleep(0.1)
        else:
            raise RuntimeError('Host readiness timed out')
        installation = api('/v1/integrations/github_actions', 'PUT', package)
        run = api('/v1/integrations/github_actions/runs', 'POST')['run_id']
        history = None
        for _ in range(400):
            history = api('/v1/integrations/github_actions/runs')
            manual = next((r for r in history['runs'] if r['id'] == run), None)
            if manual and manual['status'] == 'failed':
                raise RuntimeError('Hosted extraction failed: '+str(manual['error']))
            success = manual and manual['status'] == 'succeeded'
            scheduled = any(r['trigger']=='schedule' and r['status']=='succeeded' for r in history['runs'])
            if success and (not args.prove_schedule or scheduled):
                break
            time.sleep(0.5)
        else:
            raise RuntimeError('Hosted run timed out')
        snapshot = {'installation': installation, 'list': api('/v1/integrations'), 'history': history, 'records': api('/v1/integrations/github_actions/records')}
        encoded = json.dumps(snapshot, indent=2)
        if token in encoded:
            raise RuntimeError('Credential disclosure detected')
        (output/'snapshot.json').write_text(encoded+'\n')
        html = (ROOT/'tools/integration-host/index.html').read_text()
        embedded = json.dumps(snapshot).replace('&','\\u0026').replace('<','\\u003c').replace('>','\\u003e')
        html = html.replace('<script>', '<script id="host-snapshot" type="application/json">'+embedded+'</script><script>', 1)
        (output/'snapshot.html').write_text(html)
        counts = {kind:sum(r['record']['kind']==kind for r in snapshot['records']['records']) for kind in ['contract','fact']}
        counts['external_outcomes'] = len(history['external_outcomes'])
        counts['successful_runs'] = sum(r['status']=='succeeded' for r in history['runs'])
        print(json.dumps({'endpoint':base, 'counts':counts, 'snapshot':str(output/'snapshot.html'), 'operator_environment':str(env_path)}, indent=2), flush=True)
        if args.serve:
            print('Host remains active. Ctrl-C stops it. The API token is in the protected operator environment file.', flush=True)
            process.wait()
    except KeyboardInterrupt:
        pass
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGTERM)
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        process.stderr.close()

if __name__ == '__main__':
    main()
