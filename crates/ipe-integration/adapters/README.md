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

Runtime API sources: [Wasmi configuration](https://docs.rs/wasmi/2.0.0/wasmi/struct.Config.html)
and [store limits](https://docs.rs/wasmi/2.0.0/wasmi/struct.StoreLimitsBuilder.html).
