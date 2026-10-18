import assert from 'node:assert/strict';
import { test } from 'node:test';
import { boundedResponse, WireClient } from './external-client-transport.ts';

function client(code: string, limits = {}) {
  return new WireClient(process.execPath, ['-e', code], {}, () => {}, { timeoutMs: 1000, ...limits });
}
async function rejectedAndReaped(wire: WireClient) {
  try { await assert.rejects(wire.send('initialize', {})); }
  finally { await wire.close(); }
  assert.ok(wire.child.exitCode !== null || wire.child.signalCode !== null);
}

test('bounded response cancels oversized HTTP stream', async () => {
  let cancelled = false;
  const stream = new ReadableStream<Uint8Array>({
    start(controller) { controller.enqueue(new Uint8Array(33)); },
    cancel() { cancelled = true; },
  });
  await assert.rejects(boundedResponse(new Response(stream), 32), /response_limit/);
  assert.equal(cancelled, true);
  assert.equal(await boundedResponse(new Response('bounded'), 32), 'bounded');
});

test('wire transport reaps unresponsive and SIGTERM-ignoring child after request timeout', async () => {
  await rejectedAndReaped(client("process.on('SIGTERM',()=>{});setInterval(()=>{},1000)", { timeoutMs: 100 }));
});

test('wire transport bounds unterminated stdout and stderr floods', async () => {
  await rejectedAndReaped(client("process.stdout.write('x'.repeat(4096));setInterval(()=>{},1000)", { frameBytes: 1024 }));
  await rejectedAndReaped(client("process.stderr.write('x'.repeat(4096));setInterval(()=>{},1000)", { stderrBytes: 1024 }));
});

test('wire transport rejects malformed output, wrong IDs, and premature child exit', async () => {
  for (const output of ['{bad json}\n', '{"jsonrpc":"2.0","id":999,"result":{}}\n', 'partial']) {
    await rejectedAndReaped(client(`process.stdout.write(${JSON.stringify(output)});`));
  }
});

test('wire transport handles spawn errors without uncaught events or lingering timers', async () => {
  const wire = new WireClient('/does-not-exist/ipe-client', [], {}, () => {}, { timeoutMs: 100 });
  await assert.rejects(wire.send('initialize', {}));
  await wire.close();
});

test('wire transport accepts correlated response but force-reaps child that ignores stdin closure', async () => {
  const wire = client("process.stdin.once('data',()=>{process.stdout.write('{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\\n')});setInterval(()=>{},1000)", { timeoutMs: 100 });
  assert.deepEqual((await wire.send('initialize', {})).result, {});
  await assert.rejects(wire.finish(), /exit_timeout/);
  await wire.close();
  assert.equal(wire.child.signalCode, 'SIGKILL');
});

test('wire transport bounds aggregate output across individually valid frames', async () => {
  const wire = client("let id=0;process.stdin.on('data',()=>{process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:++id,result:{text:'x'.repeat(60)}})+'\\n')})", { frameBytes: 1024, totalBytes: 150 });
  await wire.send('first', {});
  await rejectedAndReaped(wire);
});
