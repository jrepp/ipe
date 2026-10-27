# Hosted metadata adapter ABI

The Rust Integration Workload runs a Wasmi interpreter for each discovery. The
bundled `servicenow-metadata.wat` is compiled to WASM during the Cargo build and
supplies the ServiceNow field-name interpretation previously hardcoded in Rust.
Set `adapter_path` in the private workload configuration to select another binary
WASM module at startup. The host reads and compiles that artifact once; a fresh
instance and fuel budget are allocated per discovery. Changing a file after
startup does not change the active module. Capabilities and discovery output
identify the exact bytes by SHA-256, without disclosing the path.

## `metadata-field-v1`

Exports:

- `memory`: one linear memory, no more than 65,536 bytes.
- `classify(pointer: i32, length: i32, kind: i32, required: i32) -> i32`.

The host writes an identifier of at most 128 UTF-8 bytes at offset zero, then calls
`classify`. Bytes outside that slice are not input. `kind` is 0 unknown, 1 string,
2 integer, 3 boolean, 4 reference. `required` is -1 unknown, 0 false, 1 true.
The returned role is 0 unknown, 1 lifecycle state, 2 assignee, 3 assignment group,
4 active, or 5 last updated. The host accepts only these values; guests cannot
emit strings, URLs, policy results, credentials, or arbitrary records. Roles are
hints, not verified workflow constraints. Unknown workflow semantics remain
explicit in discovery output.

Metadata identifiers are permitted guest inputs, including operator-selected
field names. Raw response bodies, provider origin, credentials, HMAC keys,
resource names, and opaque ID generation remain native. This small ABI is a
first useful interpretation extension, not the proposed general request/response
step ABI. It cannot select operations or discover workflow semantics.

## Bounds and failure behavior

No imports are accepted and no WASI interfaces are linked: guests have no ambient
network, filesystem, environment, clock, subprocess, or logging access. Module
start functions are forbidden. Binary modules are limited to 256 KiB, linear
memory to 64 KiB, call depth to 64, and value stack to 64 KiB. Tables are forbidden.
One million fuel units are shared across all fields in one discovery, including
all guest calls. The existing native request deadline and concurrency limit remain
in force. Fuel bounds synchronous guest execution; the HTTP deadline does not
preempt a synchronous Wasm call. Compilation occurs at trusted operator startup,
not on the HTTP request path.

Malformed modules fail startup with a sanitized code. A trap, exhausted fuel, or
invalid enum marks the run failed: that field and subsequent fields receive
unknown roles with `adapter_failed` or `adapter_output_invalid` coverage codes.
Native metadata observations remain available. No guest failure is reported as
successful constraint discovery. Each subsequent discovery gets a fresh instance.

The initial lifecycle uses operator startup selection. Agent upload, package
signing, staged activation, live upgrades and rollback still require the package
lifecycle design; this host does not claim those capabilities.

## `json-step-v1` JavaScript and TypeScript runtime

The separate `/v1/adapters/run` endpoint runs an operator-selected `.js` or `.ts`
source using embedded QuickJS. TypeScript types are transpiled at startup by Oxc.
The source defines a synchronous global function:

```ts
step(context, responses, state) -> { requests, state } | { outputs, state }
```

Every result must include JSON `state` and exactly one of `requests` or `outputs`.
Requests contain an id, a configured operation name, and parameters validated
against that operation's descriptor. The host executes the named HTTPS read and
resumes the guest with projected primitive response data and a native HMAC opaque
source receipt bound to the run, sequence, adapter digest, operation, query, and
response. These run-local receipts are not durable audit evidence.
The host rejects unconfigured operations, malformed parameters, and reused request
ids. The run response reports JSON outputs, step/request counts, source digest, and
`publication: false`.

The first operator configuration supports descriptors containing `name`, `table`,
`fields`, optional `parameters`, and `limit`. The descriptors are a separate explicit
read grant, independent of the discovery `resources` list. Operation routes and
selected fields are fixed in trusted configuration; guest inputs cannot supply a
URL, table, query, or credential. Configuring a script adapter requires the protected
`identity_key_env` key for receipt derivation, including when discovery resources
are also configured. The guest receives only `{connection_ref}` as context and the
responses produced during that run. Projected response fields may themselves
contain operator-selected names. It has no Node globals, npm modules, WASI, network,
filesystem, environment, or secret access. Source digest covers the language,
original source, emitted JavaScript, and the embedded SDK.

Runs are limited to eight steps, sixteen total requests, and eight requests per
step. Each serialized step result, including state and requests or outputs, is
limited to 64 KiB; combined guest input is limited to 128 KiB. QuickJS is limited to
16 MiB memory and 1 MiB stack; each step has a 100 ms deadline. Every step runs in
a fresh QuickJS context, while explicit JSON state carries values between steps.
The 100 ms deadline uses QuickJS interrupt checks and is not a hard OS process
deadline. Loading and Oxc transpilation occur in the trusted workload process at
startup, not an isolated build worker; source size is bounded, but startup does not
have a separate hard process deadline. Host broker I/O retains separate request
and response bounds. A failed run does not contaminate the next run. This is an operator configured sandbox, not a
package registry, build service, staged deployment lifecycle, or general npm
runtime.

Runtime API sources: [Wasmi configuration](https://docs.rs/wasmi/2.0.0/wasmi/struct.Config.html)
and [store limits](https://docs.rs/wasmi/2.0.0/wasmi/struct.StoreLimitsBuilder.html).
