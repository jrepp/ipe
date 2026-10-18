# RFC 012 Integration runtime and connection discovery

**Status:** Draft discovery
**Created:** 2026-10-18
**Parent and MVP:** [RFC 010](010-remediation-adapters-and-external-policy-discovery.md)

## Scope

**Initial implementation:** [standalone discovery slice](../docs/INTEGRATION_DISCOVERY.md).
M1 uses an operator-configured HTTPS origin and secret environment references,
one bearer-protected scope, a fixed metadata recipe, and bounded inventory without
preselected tables. Opaque discovered IDs use a retained private HMAC key. Guest
execution, secret enrollment UI, durable jobs and general provider packages remain
proposed below.

Own the native broker, connection onboarding, first-class discovery, sandbox execution,
and portable adapter packages inside the Integration Workload. This is one functional
area, not another workload. Agent workflow belongs to [RFC 011](011-agent-integration-toolkit.md);
domain interpretation belongs to [RFC 013](013-domain-constraints-and-projections.md).

## Credential onboarding and discovery

The first milestone accepts an operator-configured ServiceNow connection, with its
secret supplied through a protected operator path. An agent receives an opaque
connection reference, never the credential or physical origin. Initial implementation
may load a protected configuration and secret environment references at startup;
self-service secret enrollment is a later extension, not an implied shipped feature.

The agent checks the connection and asks what this credential can discover. Discovery
is bounded to explicitly authorized metadata operations and returns accessible
resource/schema descriptions, supported probe capabilities, coverage, and gaps.
Do not require the agent to choose a remediation record type before discovery.
Operator resource restrictions remain valid and are reported as coverage boundaries.
A denied metadata request means unknown visibility, not that the resource is absent.

Keep transport/credential validation, metadata access, record access, and constraint
access separate. A successful schema read proves none of the others. Select the first
business record and constraint sources from the discovery report, with authorized
samples where available. Opaque/unsupported rules remain visible as knowledge gaps.
Discovery must not run imported scripts, guess unrestricted routes, or escalate grants.

Use a lightweight ServiceNow-style HTTPS stand-in for deterministic local development.
It supplies synthetic metadata, bounded pagination, access denials and failures. It
is not proof of a deployed ServiceNow tenant's semantics; live validation pins the
actual instance release, permissions and application.

## Platform and extension ownership

| Responsibility | Native platform | Agent-authored package |
| --- | --- | --- |
| Connections | Authentication, secret rotation, TLS, destination and resource authorization | Select supported connection recipe |
| Collection | Scheduling, retries, quotas, pagination execution, checkpoints and cancellation | Operations, parameter schemas, selectors and error classification |
| Interpretation | Shared domain schemas, generated SDK, validation and replay tools | Provider identity mapping, evidence interpretation, constraint meaning and gaps |
| Publication | Opaque identities, permitted fields, deadlines, durable sequencing and delivery | Declare output types and source receipt references |
| Access | Installation grants, disclosure policy, elevation and audit | Request capabilities; never grant them |

A package combines declarative provider descriptors with bounded semantic code.
Adding an endpoint or mapping using an existing transport recipe must not require
a native release. New authentication or protocol primitives may require platform
support. Packages select pagination and incremental-sync recipes; exceptional
cursor interpretation can be a pure mapping, but iteration and durable progression
remain native responsibilities.

Capabilities are independently granted: `observe`, `discover_constraints`, and
`observe_outcomes`. Future `validate_operation` and `execute_operation` capabilities
need separate authorization and declared side effects. An endpoint named validate,
or an HTTP GET, does not establish side-effect freedom. Workflow action execution
is outside the first implementation scope.

## Adapter execution

An immutable package contains a manifest, source and built artifact, protocol and
schema versions, capability requests, dependency locks, fixtures, expected outputs,
and build provenance. An authorized installation binds its digest to an opaque
scope, connection capabilities, resource grants, schedules, and output destinations.
Saving or testing a package does not activate it. Activation may be automated within
existing authority; this design does not require manual approval of every package.

[RFC 011](011-agent-integration-toolkit.md) specifies the agent-facing toolkit for
probing, replay, staging, deployment, upgrades, and monitoring over these boundaries.

The initial guest interface is a bounded step:
`step(context, responses, state) -> requests | outputs`.
Requests select named descriptor operations and validated parameters. The host
executes them and resumes the guest. State and request counts are bounded. This
keeps authentication, asynchronous I/O, and retry machinery out of adapter code.

Guests receive only permitted response fields and opaque source receipts. They
have no ambient network, filesystem, environment, subprocess, or secret access.
The broker checks final destinations, resolved addresses, encoded parameters,
redirects, pagination links, and resources within a provider host. It limits
response decompression and parsing as well as request volume. Source text remains
untrusted data, including remediation instructions and policy descriptions.

Sandbox limits cover CPU, wall time, memory, stack, state, output, and logs. Host
I/O has independent deadlines. Build workers are also restricted, use pinned
inputs, and have no production credentials. Package identity proves what ran, not
whether its interpretation was correct. Installation revocation fences old runs;
rollback changes the package selection without rewriting evidence history.

**Execution hypothesis:** TypeScript authored against a small generated SDK,
transpiled to JavaScript and executed in Wasm, hosted by the native Rust runtime.
[Javy](https://github.com/bytecodealliance/javy) supplies a JavaScript-in-Wasm path;
[Wasmtime](https://docs.wasmtime.dev/security.html) supplies explicit host interfaces
and [execution interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html).
Compare it with a restricted [QuickJS](https://bellard.org/quickjs/) worker using
the same contract. Deno, Python workers, and Rust-to-Wasm remain alternatives if
those experiments fail. Node APIs and arbitrary npm compatibility are not assumed;
[Node vm](https://nodejs.org/api/vm.html) is not a standalone security boundary.
Measure authoring effort, compatibility, diagnosis, latency, memory, and build cost
before selecting an engine.

## Area roadmap

| Phase | Deliverable | Proof |
| --- | --- | --- |
| M1 onboarding and discovery | Standalone workload, configured credential, opaque handle, bounded metadata discovery | CLI/MCP agent learns permitted schemas and gaps against HTTPS stand-in; no secret/topology disclosure |
| M2 adapter iteration | One guest engine, package manifest, immutable build, fixed native recipes | Agent modifies only mappings/descriptors; replay and resource-limit tests pass |
| M3 first live integration | Scoped ServiceNow reads for a discovery-selected record and constraint source | Actual provider responses mapped with explicit coverage and no arbitrary networking |
| M4 operational proof | Fencing, cancellation and bounded execution under upgrades | Old runs cannot publish after cutover; workload survives malformed upstream data |
| Later | Additional protocol/auth recipes, self-service enrollment, second provider | New provider using existing recipes requires no native code change |

## Review questions

- Which metadata operations are supported by the first credential, and what can be safely exposed?
- Which guest engine meets isolation, authoring and deployment budgets?
- Which provider behaviors need reusable recipes rather than semantic code?
