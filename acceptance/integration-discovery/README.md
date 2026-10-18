# Integration discovery vertical-slice acceptance

Build and run the separate Integration Workload against a local HTTPS ServiceNow
stand-in, without provider credentials or an enforcement workload:

```sh
cargo build -p ipe-integration
python3 scripts/accept-integration-discovery.py
```

The suite launches the real binary and exercises HTTP, the agent CLI, and MCP
stdio. It writes `target/integration-discovery-report.json`, exits nonzero on a
failure, and tears down temporary processes, certificates, and configuration.
`--binary PATH` and `--report PATH` override the defaults. Python 3 and `openssl`
are the only harness dependencies.

Checks cover authentication, wrong connection references, strict request
parameters, no publication or enforcement route, bounded metadata queries,
provider authentication, opaque identities across restart, and discovery/report/log
redaction. Both configured candidates and credential-first inventory without known
tables are exercised. A delayed HTTPS fixture verifies the cumulative run deadline,
concurrent-run rejection, and permit release after cancellation. Hostile provider fields contain synthetic secret, topology, and source
identifier markers. Provider faults include access denied, unauthorized, 500,
404, empty results, wrong envelope shape, malformed JSON, oversized body,
redirects, and full row-limit responses. Unavailable evidence must remain unknown;
metadata alone must never imply that all workflow constraints were discovered.

For a small deployment demonstration:

```sh
python3 scripts/demo-integration-discovery.py --build
python3 scripts/demo-integration-discovery.py --serve
```

The demonstration exercises API, CLI, and MCP against the launched workload.
The default stops after writing `target/integration-demo-report.json`.
`--serve` retains the local deployment until Ctrl-C, prints its local API address
and opaque connection reference, and gives the operator a private environment
file containing a synthetic agent token. Source that file before using
`scripts/integration-toolkit.py --url <printed API address> capabilities`.
All generated files are temporary and removed on shutdown.

These tests prove the configured single-connection process boundary. They do not
claim multi-tenant authorization, production secret storage, ServiceNow OAuth
onboarding, live-instance ACL compatibility, persisted jobs, arbitrary adapter
sandbox execution, or enforcement publication. Restart verifies stable configured
identities, not durable discovery history. The fixture does not emulate ServiceNow
business rules, ACLs, application licensing, or workflow semantics. Instance
validation remains a separate milestone with authorized credentials.
