# TypeScript integration test facades

Run `node testing/integration/servicenow-standin.ts` with Node 22.18+ and
`openssl`. The facade has no runtime package dependencies. It starts a loopback
HTTPS ServiceNow stand-in, generates a temporary CA and server certificate, and
prints one JSON readiness line:

```json
{"origin":"https://127.0.0.1:PORT","ca_cert_path":"/temporary/ca.crt","control_url":"http://127.0.0.1:PORT","control_token":"random-fixture-control-token","mode":"success"}
```

Stop with Ctrl-C or SIGTERM. Temporary certificates are removed on shutdown.
`--directory PATH` retains certificates and keys in an operator-selected fixture
directory. `--mode MODE` selects an initial fault. Never supply production
credentials. The provider requires the synthetic Bearer token
`provider-secret-DO-NOT-DISCLOSE`; only the native workload receives this token.

The facade provides synthetic `sys_db_object`, `sys_dictionary`, and `sys_choice`
Table API envelopes, including hostile labels, internal-looking links, and raw
identities for disclosure tests. It does not emulate ServiceNow ACLs, OAuth,
business rules, or licensing. Passing fixture tests proves the local contract,
not compatibility with a particular ServiceNow instance.

The separate loopback HTTP control server is test-only and requires
`Authorization: Bearer <control_token>`. `GET /state` returns
`{mode, delay_seconds, requests:[{path,headers}]}`. `POST /control` accepts any
subset of `{mode, delay_seconds, expected_token, clear_requests}` and returns the
same state. Control credentials and recorded provider headers belong only to the
trusted test harness; never project them into agent or policy outputs.

Modes: `success`, `empty`, `truncated`, `wrong_shape`, `denied`, `unauthorized`,
`failure`, `unsupported`, `redirect`, `malformed`, `oversize`. Delay supports
concurrency and cancellation tests. Credential rotation tests use `expected_token`.

For development:

```sh
cd testing/integration
npm ci
npm run check
npm test
npm run build
node dist/servicenow-standin.js
```

TypeScript and Node declarations are development-only dependencies. The Rust
acceptance runner launches the `.ts` facade directly, requiring no npm install.

## Independent external agent client

`external-agent-client.ts` probes an already-running **synthetic discovery demo**
through CLI subprocesses, direct HTTP, and a hand-written JSON-RPC MCP client.
It imports neither the workload implementation nor its MCP SDK. It reads only the
explicit agent token handoff file and the two executable files for SHA256 hashing;
it does not inspect operator configuration or provider credentials.

```sh
node testing/integration/external-agent-client.ts \
  --url http://127.0.0.1:PORT \
  --token-file /private/demo/agent.env \
  --toolkit target/debug/integration-toolkit \
  --workload target/debug/ipe-integration \
  --connection-ref conn_acceptance \
  --report target/external-agent-validation.json
```

The client permits only loopback HTTP and uses the demo's synthetic metadata
expectations. The report contains sanitized results, observation time, executable
hashes and observed adapter digest. An optional `--source-revision FULL_COMMIT_HASH`
records the operator-supplied build revision; the client cannot prove that a binary
was built from that revision. It validates masking, bounded discovery and explicit
unknown workflow semantics, then rejection of invalid authentication, foreign
connections, URL injection and unknown MCP tools. Public JSON Schema URLs are
allowed in tool schemas; provider URLs and known hostile markers are forbidden.
It stops its own MCP subprocess and leaves the deployment running. This is a
same-machine external-client check, not a separate OS sandbox or live ServiceNow
compatibility suite.
