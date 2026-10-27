# RFC 012 Integration runtime and connection discovery

**Status:** Draft discovery
**Created:** 2026-10-18
**Parent and MVP:** [RFC 010](010-remediation-adapters-and-external-policy-discovery.md)

## Scope

**Initial implementation:** [standalone discovery slice](../docs/INTEGRATION_DISCOVERY.md).
M1 uses an operator-configured HTTPS origin and secret environment references,
one bearer-protected scope, a fixed metadata recipe, and bounded inventory without
preselected tables. Opaque discovered IDs use a retained private HMAC key. The Rust
workload hosts Wasmi for bounded metadata interpretation, with a bundled WASM
mapping and optional operator-selected artifact at startup. The CLI/MCP client and
acceptance/deployment harness are Rust; test provider facades are TypeScript.
Secret enrollment UI, durable jobs and general provider packages remain proposed.

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

Coverage diagnostics accumulate within each declared scope. Resource enumeration
and each resource's field enumeration keep separate, deduplicated sets of
host-generated failure codes. Reaching a row limit, malformed/duplicate metadata,
unsupported types and guest failures can occur together; reporting one must not
erase another. A scalar summary may prioritize truncation, but cannot replace the
full issue set. An observed resource does not imply its fields are complete; an
empty issue set means only that no failure was observed within that bounded read.
The discovery slice implements this with `coverage_codes` and per-resource
`field_coverage_codes`, alongside the existing scalar summaries. These diagnostics
never confer negative coverage or permission to publish a complete snapshot.

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

The proposed general guest interface is a bounded step:
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

**Implemented engines and ABIs:** [Wasmi](https://docs.rs/wasmi/2.0.0/wasmi/) hosts
the metadata interpreter in [Rust](../crates/ipe-integration/src/wasm.rs). Its
[metadata-field-v1 ABI](../crates/ipe-integration/adapters/README.md) exports memory
and `classify(pointer, length, kind, required) -> role`. Inputs are bounded permitted
field identifiers and normalized metadata; outputs are a finite validated
semantic-role enum. Transport, provider resource selection, credentials, identity
derivation and public output construction remain native. Guest failures produce
explicit unknown roles and coverage codes, never workflow constraints.

The default ServiceNow mapping is WAT compiled to WASM at Cargo build time. An
operator may select a binary WASM module with `adapter_path`; the workload loads
it once at startup and reports its SHA-256 digest. Each discovery gets a fresh
instance with one million fuel units shared across field calls, 64 KiB memory,
64 KiB value stack and call depth 64. Modules are capped at 256 KiB; tables, imports
and start functions are forbidden. No WASI interfaces are linked. Fuel bounds
synchronous execution; the native HTTP deadline does not preempt a synchronous
WASM call. Compilation happens at operator startup rather than on the request path.

The workload also implements a bounded `json-step-v1` ABI using embedded QuickJS
with Oxc TypeScript transpilation. An operator-selected `.js` or `.ts` source defines
a synchronous global `step(context, responses, state)`. Each call returns JSON state
and exactly one of named operation requests or JSON outputs. The host broker
validates operation descriptors and parameters, performs HTTPS reads, and resumes
the guest with selected primitive response fields and a native HMAC opaque source
receipt bound to run, request sequence, adapter digest, operation, query, and
response. These run-local receipts are not durable audit evidence.
Guests have no Node APIs, npm modules, WASI, ambient network, filesystem, environment,
or secret access. Projected fields may contain operator-selected names. The source
digest covers source language, original source, emitted JavaScript, and the embedded SDK. This step surface is
currently configured by the operator and has no package registry, agent upload,
staging, signing, automated activation, or publication lifecycle. It does not promise
arbitrary JavaScript or npm compatibility.

Its `script_operations` descriptors are explicit read grants, independent of the
discovery `resources` list. Configuring a script adapter requires the protected
identity key for receipt derivation, even when discovery resources are also set.

The step contract is bounded to eight steps, sixteen total requests, eight requests
per step, 64 KiB for each serialized step result including state, 128 KiB combined
guest input, 16 MiB QuickJS memory, 1 MiB stack, and a 100 ms per-step deadline.
Each step evaluates in a fresh QuickJS context, with JSON state explicitly passed
between steps. Host I/O has independent deadlines and response bounds. JavaScript
and typed TypeScript acceptance fixtures exercise real
broker reads, undeclared operation and parameter rejection, execution timeout, and
fresh-run recovery. This is an initial local execution proof, not evidence of build
worker isolation or general package lifecycle authority.

The 100 ms step deadline relies on QuickJS interrupt checks; it is not a hard OS
process deadline. Source loading and TypeScript transpilation run in the trusted
workload process at operator startup, not an isolated build worker. Source size is
bounded, but startup has no separate hard process deadline.

No Node APIs, arbitrary npm compatibility, or WASI access are promised. Extending
beyond these ABIs requires separate design and evidence for build isolation,
semantic coverage and lifecycle authority.

## Area roadmap

| Phase | Deliverable | Proof |
| --- | --- | --- |
| M1 onboarding and discovery | Standalone workload, configured credential, opaque handle, bounded metadata discovery | CLI/MCP agent learns permitted schemas and gaps against HTTPS stand-in; no secret/topology disclosure |
| M2 adapter iteration | Wasmi metadata host and operator-selected bounded `json-step-v1` JS/TS runtime implemented; package manifest, general package lifecycle, immutable agent build and staging remain proposed | Sandbox and JS/TS broker-read acceptance pass; still require agent edits/replay/stage proof without native changes |
| M3 first live integration | Scoped ServiceNow reads for a discovery-selected record and constraint source | Actual provider responses mapped with explicit coverage and no arbitrary networking |
| M4 operational proof | Fencing, cancellation and bounded execution under upgrades | Old runs cannot publish after cutover; workload survives malformed upstream data |
| Later | Additional protocol/auth recipes, self-service enrollment, second provider | New provider using existing recipes requires no native code change |

## Review questions

- Which metadata operations are supported by the first credential, and what can be safely exposed?
- Does the selected Wasmi host meet production latency and authoring budgets as the ABI grows?
- Which provider behaviors need reusable recipes rather than semantic code?
