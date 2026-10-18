import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { request } from 'node:https';
import { test } from 'node:test';
import { MODES, RAW_ID, SECRET, TOPOLOGY, startFixture } from './servicenow-standin.ts';

test('HTTPS facade authenticates, provides hostile metadata, injects faults, and cleans up', async () => {
  const fixture = await startFixture();
  const ca = readFileSync(fixture.ca_cert_path);
  const get = (path: string, token = SECRET) => new Promise<{ status: number; body: string; location?: string }>((resolve, reject) => {
    const req = request(fixture.origin + path, { ca, headers: { authorization: `Bearer ${token}` } }, response => {
      let body = '';
      response.setEncoding('utf8'); response.on('data', chunk => { body += chunk; });
      response.on('end', () => resolve({ status: response.statusCode!, body, location: response.headers.location }));
    });
    req.on('error', reject); req.end();
  });
  const control = async (value: unknown) => fetch(fixture.control_url + '/control', { method: 'POST', headers: { authorization: `Bearer ${fixture.control_token}` }, body: JSON.stringify(value) });
  try {
    assert.equal((await fetch(fixture.control_url + '/state')).status, 401);
    assert.equal((await get('/api/now/table/sys_db_object', 'wrong')).status, 401);
    const metadata = await get('/api/now/table/sys_db_object');
    assert.equal(metadata.status, 200);
    for (const marker of [SECRET, RAW_ID, TOPOLOGY]) assert.ok(metadata.body.includes(marker));
    assert.equal(JSON.parse((await get('/api/now/table/sys_dictionary')).body).result[0].mandatory, 'true');
    assert.equal((await control({ mode: 'arbitrary' })).status, 400);
    assert.equal((await control({ delay_seconds: -1 })).status, 400);
    assert.equal((await control({ invented: true })).status, 400);
    for (const mode of MODES) {
      assert.equal((await control({ mode })).status, 200);
      const result = await get('/api/now/table/sys_db_object?sysparm_limit=9');
      const statuses: Record<string, number> = { denied: 403, unauthorized: 401, failure: 500, unsupported: 404, redirect: 302 };
      assert.equal(result.status, statuses[mode] ?? 200, mode);
      if (mode === 'truncated') assert.equal(JSON.parse(result.body).result.length, 9);
      if (mode === 'empty') assert.deepEqual(JSON.parse(result.body).result, []);
      if (mode === 'wrong_shape') assert.equal(Array.isArray(JSON.parse(result.body).result), false);
      if (mode === 'oversize') assert.equal(result.body.length, 2 * 1024 * 1024);
      if (mode === 'malformed') assert.throws(() => JSON.parse(result.body));
      if (mode === 'redirect') assert.equal(result.location, fixture.origin + '/forbidden-redirect');
    }
    await control({ mode: 'success', expected_token: 'rotated', clear_requests: true, delay_seconds: 0.03 });
    const started = Date.now();
    assert.equal((await get('/api/now/table/sys_dictionary')).status, 401);
    assert.ok(Date.now() - started >= 25);
    assert.equal((await get('/api/now/table/sys_dictionary', 'rotated')).status, 200);
    const state = await (await fetch(fixture.control_url + '/state', { headers: { authorization: `Bearer ${fixture.control_token}` } })).json();
    assert.equal(state.requests.length, 2);
    assert.equal(state.requests[1].headers.authorization, 'Bearer rotated');
    assert.equal(state.requests[1].path, '/api/now/table/sys_dictionary');
  } finally { await fixture.stop(); }
  assert.equal(existsSync(fixture.ca_cert_path), false);
});
