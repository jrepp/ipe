import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { boundedResponse, RESPONSE_LIMIT, WireClient } from './external-client-transport.ts';
import { parseArgs } from 'node:util';
// Independent external wire client: never imports workload or MCP SDK implementation.
const { values } = parseArgs({ options: {
  url: { type: 'string' }, 'token-file': { type: 'string' }, toolkit: { type: 'string' },
  workload: { type: 'string' }, report: { type: 'string' },
  'connection-ref': { type: 'string', default: 'conn_acceptance' },
  'source-revision': { type: 'string' },
} });
if (!values.url || !values['token-file'] || !values.toolkit || !values.workload || !values.report) throw Error('required: --url --token-file --toolkit --workload --report');
const binary = values.toolkit!;
const endpoint = values.url!;
const parsedEndpoint = new URL(endpoint);
if (parsedEndpoint.protocol !== 'http:' || parsedEndpoint.hostname !== '127.0.0.1' || parsedEndpoint.pathname !== '/' || parsedEndpoint.search || parsedEndpoint.hash || parsedEndpoint.username || parsedEndpoint.password) throw Error('only local loopback deployment accepted');
const credential = readFileSync(values['token-file']!, 'utf8');
const parsedToken = credential.match(/^export IPE_INTEGRATION_TOKEN='([^']+)'\s*$/)?.[1];
if (!parsedToken) throw Error('agent credential format invalid');
const token: string = parsedToken;
const connectionRef = values['connection-ref'];
const report: Record<string, any> & { operations: Record<string, unknown>[] } = {
  validation: 'independent external-client deployment exploration',
  observed_at: new Date().toISOString(),
  scope: 'Same-machine deterministic external client against an existing Rust deployment and synthetic TypeScript ServiceNow facade; no separate OS sandbox, autonomous reasoning claim, or real ServiceNow validation.',
  method: 'CLI subprocesses with agent-only environment; direct authenticated HTTP; independent Node JSON-RPC line client, sequential initialize then initialized notification then tool requests, no rmcp SDK client.',
  disclosure_check: 'Agent token, synthetic provider secret/topology/raw identity markers, physical table identifiers and physical URLs rejected; public json-schema.org schema references permitted.',
  hash_scope: 'SHA256 identifies supplied executable files. The client launches the supplied toolkit, but does not attest that the existing HTTP workload process runs the supplied workload file.',
  toolkit_binary_sha256: createHash('sha256').update(readFileSync(binary)).digest('hex'),
  workload_binary_sha256: createHash('sha256').update(readFileSync(values.workload!)).digest('hex'),
  operations: [],
};
if (values['source-revision']) {
  if (!/^[a-f0-9]{40}$/.test(values['source-revision'])) throw Error('source revision must be a full commit hash');
  report.source_revision = values['source-revision'];
}
let activeName = '';
function require(condition: unknown, message: string) { if (!condition) throw Error(message); }
function safe(value: unknown) {
  const text = typeof value === 'string' ? value : JSON.stringify(value);
  for (const marker of [token, 'provider-secret-DO-NOT-DISCLOSE', 'internal-remediation.corp.example', '1234567890abcdef1234567890abcdef', 'sn_vul_vulnerable_item', 'sys_dictionary', 'sys_db_object']) require(!text.includes(marker), 'response disclosure detected');
  const withoutPublicSchemaReferences = text.replace(/https?:\/\/json-schema\.org\/[^"\s]*/g, 'public-schema-reference');
  require(!/https?:\/\//.test(withoutPublicSchemaReferences), 'response contains physical URL');
}
async function check(name: string, operation: () => Promise<unknown>) {
  activeName = name;
  const details = await operation(); safe(details);
  report.operations.push({name, outcome:'passed', details});
}
function cli(args: string[], bearer = token!) {
  const output = spawnSync(binary, ['--url',endpoint,...args], {env:{IPE_INTEGRATION_TOKEN:bearer}, encoding:'utf8', timeout:15000, maxBuffer:RESPONSE_LIMIT, killSignal:'SIGKILL'});
  require(!output.error && output.signal === null, 'CLI process failed');
  safe(output.stdout); safe(output.stderr);
  return output;
}
async function http(method: string, route: string, body: unknown, bearer = token!) {
  const response = await fetch(endpoint+route, {method, redirect:'manual', headers:{authorization:`Bearer ${bearer}`, 'content-type':'application/json'}, ...(body ? {body:JSON.stringify(body)} : {}), signal:AbortSignal.timeout(15000)});
  const raw = await boundedResponse(response); safe(raw);
  return {status:response.status, body:raw ? JSON.parse(raw) : null};
}
let wire: WireClient | undefined;
try {
  await check('CLI capabilities', async () => {
    const output = cli(['capabilities']); require(output.status === 0,'CLI capabilities failed');
    const value = JSON.parse(output.stdout); require(value.publication === false,'publication enabled');
    require(value.adapter.engine === 'wasmi','WASM engine absent'); report.observed_adapter_digest = value.adapter.digest;
    return value;
  });
  await check('CLI connection check', async () => {
    const output = cli(['check',connectionRef]); require(output.status === 0,'CLI check failed');
    const value = JSON.parse(output.stdout); require(value.status === 'available','connection unavailable'); return value;
  });
  await check('CLI discovery and constraint gaps', async () => {
    const output = cli(['discover',connectionRef]); require(output.status === 0,'CLI discovery failed');
    const value = JSON.parse(output.stdout); require(value.coverage === 'bounded' && value.constraints.status === 'unknown','coverage overclaimed');
    require(value.resources.length === 1 && value.resources[0].resource_ref.startsWith('resource_'),'opaque inventory absent');
    require(value.resources[0].fields[0].role === 'lifecycle_state','WASM classification absent'); return value;
  });
  await check('CLI bad agent token', async () => {
    const output = cli(['discover',connectionRef],'invalid-agent-token'); require(output.status === 1 && JSON.parse(output.stdout).error?.code === 'unauthenticated','incorrect bad-token rejection');
    return {exit_code:output.status, stdout:output.stdout.trim(), stderr:output.stderr.trim()};
  });
  await check('HTTP authenticated discovery', async () => {
    const result = await http('POST','/v1/discovery',{connection_ref:connectionRef});
    require(result.status === 200 && result.body.constraints.status === 'unknown','HTTP discovery failed');
    return {status:result.status,coverage:result.body.coverage,constraints:result.body.constraints};
  });
  for (const [name,body,bearer,expectedStatus,expectedCode] of [
    ['HTTP bad token',{connection_ref:connectionRef},'invalid-agent-token',401,'unauthorized'],
    ['HTTP foreign reference',{connection_ref:'conn_foreign'},token,404,'connection_unavailable'],
    ['HTTP injected URL',{connection_ref:connectionRef,url:'https://attacker.invalid'},token,400,'invalid_request'],
  ] as const) await check(name, async () => { const result = await http('POST','/v1/discovery',body,bearer); require(result.status === expectedStatus && result.body.code === expectedCode,'incorrect HTTP rejection'); return result; });
  wire = new WireClient(binary,['--url',endpoint,'mcp'],{IPE_INTEGRATION_TOKEN:token},safe);
  const mcp = wire;
  const send = (method: string, params: unknown) => mcp.send(method, params);
  await check('Independent JSON-RPC MCP initialization',async()=>{
    const value=await send('initialize',{protocolVersion:'2025-11-25',capabilities:{},clientInfo:{name:'independent-agent-probe',version:'1'}});
    require(value.result?.protocolVersion === '2025-11-25','MCP negotiation failed');
    require(value.result?.serverInfo?.name === 'ipe-integration-toolkit','MCP server identity incorrect');
    mcp.notify('notifications/initialized');return value.result;
  });
  await check('MCP tool listing',async()=>{
    const value=await send('tools/list',{}); const names=value.result?.tools.map((t: {name: string})=>t.name).sort();
    require(JSON.stringify(names)===JSON.stringify(['integration_capabilities','integration_check_connection','integration_discover','integration_run_adapter']),'unexpected MCP tools');return {names};
  });
  for(const [name,args] of [['integration_capabilities',{}],['integration_check_connection',{connection_ref:connectionRef}],['integration_discover',{connection_ref:connectionRef}]]) {
    await check(`MCP ${name}`,async()=>{
      const value=await send('tools/call',{name,arguments:args});require(value.result && !value.result.isError,'MCP call failed');
      require(value.result.structuredContent,'MCP structured output absent');return value.result.structuredContent;
    });
  }
  for(const [label,name,args] of [
    ['unknown tool','integration_arbitrary',{}],
    ['foreign reference','integration_discover',{connection_ref:'conn_foreign'}],
    ['injected URL','integration_discover',{connection_ref:connectionRef,url:'https://attacker.invalid'}]
  ]) await check(`MCP rejects ${label}`,async()=>{
    const value=await send('tools/call',{name,arguments:args});
    if (label === 'unknown tool') require(value.error?.code === -32602, 'incorrect unknown-tool rejection');
    else require(value.result?.isError === true && value.result.structuredContent?.error?.code === (label === 'foreign reference' ? 'not_found' : 'invalid_tool_arguments'), 'incorrect MCP rejection');
    return value;
  });
  await mcp.finish();
  report.operations.push({name:'MCP protocol-only stdout and clean teardown',outcome:'passed'});
  require(createHash('sha256').update(readFileSync(binary)).digest('hex') === report.toolkit_binary_sha256, 'toolkit binary changed during validation');
  require(createHash('sha256').update(readFileSync(values.workload!)).digest('hex') === report.workload_binary_sha256, 'workload binary changed during validation');
  report.passed=true;
} catch(error) {
  report.passed=false;report.failure={operation:activeName,message:'validation_failed'};
} finally {
  try { await wire?.close(); }
  catch { report.passed=false;report.failure={operation:'MCP cleanup',message:'validation_failed'}; }
  safe(report);
  writeFileSync(values.report!,JSON.stringify(report,null,2)+'\n');
  process.stdout.write(JSON.stringify({passed:report.passed,operations:report.operations.length,failure:report.failure,toolkit_binary_sha256:report.toolkit_binary_sha256,workload_binary_sha256:report.workload_binary_sha256,adapter_digest:report.observed_adapter_digest})+'\n');
  if (!report.passed) process.exitCode = 1;
}
