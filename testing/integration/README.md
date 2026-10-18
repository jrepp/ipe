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
