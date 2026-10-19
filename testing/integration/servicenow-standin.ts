/** Synthetic HTTPS provider facade; never use production credentials. */
import { randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer as httpServer } from 'node:http';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { createServer } from 'node:https';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import type { AddressInfo } from 'node:net';
export const SECRET = 'provider-secret-DO-NOT-DISCLOSE';
export const TOPOLOGY = 'internal-remediation.corp.example';
export const RAW_ID = '1234567890abcdef1234567890abcdef';
export const MODES = ['success', 'empty', 'truncated', 'mixed_limits', 'wrong_shape', 'denied', 'unauthorized', 'failure', 'unsupported', 'redirect', 'malformed', 'oversize'] as const;
type Mode = typeof MODES[number];
type RequestRecord = { path: string; headers: IncomingMessage['headers'] };
function certificates(directory: string) {
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  const ca = join(directory, 'ca.crt'), config = join(directory, 'tls.cnf'), ext = join(directory, 'server.ext');
  writeFileSync(config, '[req]\ndistinguished_name=dn\nx509_extensions=ext\n[dn]\n[ext]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n');
  writeFileSync(ext, 'subjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n');
  for (const args of [
    ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1', '-keyout', join(directory, 'ca.key'), '-out', ca, '-subj', '/CN=Fixture CA', '-config', config],
    ['req', '-newkey', 'rsa:2048', '-nodes', '-keyout', join(directory, 'server.key'), '-out', join(directory, 'server.csr'), '-subj', '/CN=localhost'],
    ['x509', '-req', '-in', join(directory, 'server.csr'), '-CA', ca, '-CAkey', join(directory, 'ca.key'), '-CAcreateserial', '-days', '1', '-extfile', ext, '-out', join(directory, 'server.crt')],
  ]) execFileSync('openssl', args, { stdio: 'pipe' });
  return { ca, key: readFileSync(join(directory, 'server.key')), cert: readFileSync(join(directory, 'server.crt')) };
}
function json(response: ServerResponse, status: number, value: unknown) {
  response.writeHead(status, { 'content-type': 'application/json' }); response.end(JSON.stringify(value));
}
export async function startFixture(options: { directory?: string; mode?: Mode } = {}) {
  const temporary = !options.directory;
  const directory = options.directory ? resolve(options.directory) : mkdtempSync(join(tmpdir(), 'ipe-servicenow-standin-'));
  const tls = certificates(directory);
  let mode: Mode = options.mode ?? 'success', delaySeconds = 0, expectedToken = SECRET;
  let requests: RequestRecord[] = [];
  const controlToken = randomBytes(32).toString('hex');
  let origin = '';
  const timers = new Set<ReturnType<typeof setTimeout>>();
  function payload(path: string): string {
    if (mode === 'malformed') return '{"result": [';
    if (mode === 'oversize') return 'x'.repeat(2 * 1024 * 1024);
    if (!['success', 'empty', 'truncated', 'mixed_limits', 'wrong_shape'].includes(mode)) return JSON.stringify({ error: { message: SECRET, detail: TOPOLOGY } });
    if (mode === 'wrong_shape') return JSON.stringify({ result: { secret: SECRET } });
    if (mode === 'empty') return '{"result":[]}';
    const parsed = new URL(path, 'https://localhost');
    const tables: Record<string, object[]> = {
      sys_db_object: [{ sys_id: RAW_ID, name: 'sn_vul_vulnerable_item', label: `${SECRET} at https://${TOPOLOGY}`, super_class: { value: RAW_ID, link: `https://${TOPOLOGY}/api/now/table/task/${RAW_ID}` } }],
      sys_dictionary: [{ sys_id: RAW_ID, name: 'sn_vul_vulnerable_item', element: 'state', internal_type: 'choice', mandatory: 'true', read_only: 'false', column_label: SECRET, reference: TOPOLOGY }],
      sys_choice: [{ sys_id: RAW_ID, name: 'sn_vul_vulnerable_item', element: 'state', value: '1', label: SECRET, inactive: 'false' }],
    };
    let rows = tables[parsed.pathname.split('/').at(-1) ?? ''] ?? [];
    if (mode === 'mixed_limits') {
      const table = parsed.pathname.split('/').at(-1);
      const rows = table === 'sys_db_object'
        ? Array.from({ length: 9 }, (_, i) => ({ name: i === 0 ? 'invalid^table' : `table_${i === 2 ? 1 : i}` }))
        : Array.from({ length: 33 }, (_, i) => ({ element: i === 0 ? 'invalid^field' : `field_${i === 2 ? 1 : i}`, internal_type: i === 3 ? 'unsupported' : 'string', mandatory: false }));
      return JSON.stringify({ result: rows });
    }

    if (mode === 'truncated') {
      const limit = Math.min(1000, Math.max(1, Number(parsed.searchParams.get('sysparm_limit') ?? 100) || 100));
      rows = Array.from({ length: limit }, () => rows[0] ?? { sys_id: RAW_ID });
    }
    return JSON.stringify({ result: rows });
  }
  const server = createServer({ key: tls.key, cert: tls.cert }, (request, response) => {
    requests.push({ path: request.url ?? '/', headers: request.headers });
    const send = () => {
      let status = ({ denied: 403, unauthorized: 401, failure: 500, unsupported: 404, redirect: 302 } as Partial<Record<Mode, number>>)[mode] ?? 200;
      let body = payload(request.url ?? '/');
      if (request.headers.authorization !== `Bearer ${expectedToken}`) { status = 401; body = '{"error":{"message":"authentication required"}}'; }
      response.writeHead(status, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body), ...(mode === 'redirect' ? { location: `${origin}/forbidden-redirect` } : {}) });
      response.end(body);
    };
    if (delaySeconds === 0) send();
    else { const timer = setTimeout(() => { timers.delete(timer); send(); }, delaySeconds * 1000); timers.add(timer); }
  });
  const control = httpServer(async (request, response) => {
    if (request.headers.authorization !== `Bearer ${controlToken}`) return json(response, 401, { error: 'unauthorized' });
    if (request.method === 'GET' && request.url === '/state') return json(response, 200, { mode, delay_seconds: delaySeconds, requests });
    if (request.method !== 'POST' || request.url !== '/control') return json(response, 404, { error: 'not_found' });
    try {
      let body = '';
      for await (const chunk of request) { body += chunk; if (body.length > 4096) return json(response, 413, { error: 'too_large' }); }
      const value: unknown = JSON.parse(body);
      if (!value || typeof value !== 'object' || Array.isArray(value)) throw Error('invalid');
      const input = value as Record<string, unknown>;
      if (Object.keys(input).some(key => !['mode', 'delay_seconds', 'expected_token', 'clear_requests'].includes(key))) throw Error('invalid');
      if (input.mode !== undefined && !MODES.includes(input.mode as Mode)) throw Error('invalid');
      if (input.delay_seconds !== undefined && (typeof input.delay_seconds !== 'number' || !Number.isFinite(input.delay_seconds) || input.delay_seconds < 0 || input.delay_seconds > 30)) throw Error('invalid');
      if (input.expected_token !== undefined && (typeof input.expected_token !== 'string' || input.expected_token.length > 1024)) throw Error('invalid');
      if (input.clear_requests !== undefined && typeof input.clear_requests !== 'boolean') throw Error('invalid');
      if (input.mode !== undefined) mode = input.mode as Mode;
      if (input.delay_seconds !== undefined) delaySeconds = input.delay_seconds as number;
      if (input.expected_token !== undefined) expectedToken = input.expected_token as string;
      if (input.clear_requests) requests = [];
      json(response, 200, { mode, delay_seconds: delaySeconds, requests });
    } catch { json(response, 400, { error: 'invalid_control' }); }
  });
  for (const listener of [server, control]) await new Promise<void>((resolve, reject) => { listener.once('error', reject); listener.listen(0, '127.0.0.1', resolve); });
  origin = `https://127.0.0.1:${(server.address() as AddressInfo).port}`;
  return {
    origin, ca_cert_path: tls.ca, control_url: `http://127.0.0.1:${(control.address() as AddressInfo).port}`, control_token: controlToken, mode,
    async stop() {
      for (const timer of timers) clearTimeout(timer);
      await Promise.all([server, control].map(listener => new Promise<void>(resolve => { listener.close(() => resolve()); listener.closeAllConnections(); })));
      if (temporary) rmSync(directory, { recursive: true, force: true });
    },
  };
}
async function main() {
  const options: { directory?: string; mode?: Mode } = {};
  const args = process.argv.slice(2);
  while (args.length) {
    const flag = args.shift(), value = args.shift();
    if (flag === '--directory' && value) options.directory = value;
    else if (flag === '--mode' && value && MODES.includes(value as Mode)) options.mode = value as Mode;
    else throw Error('usage: servicenow-standin.ts [--directory PATH] [--mode MODE]');
  }
  const fixture = await startFixture(options);
  process.stdout.write(`${JSON.stringify(fixture)}\n`);
  for (const signal of ['SIGTERM', 'SIGINT'] as const) process.once(signal, () => { void fixture.stop(); });
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch(() => { process.stderr.write('stand-in startup failed\n'); process.exitCode = 1; });
}
