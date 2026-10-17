# Runtime data HTTP acceptance

Run the runtime facts acceptance suite against a built `ipe-server` binary:

```sh
cargo build -p ipe-server
python3 scripts/accept-runtime-data.py
```

`make runtime-acceptance` builds and runs the same workflow. The
`Runtime data acceptance` CI workflow runs it on pull requests and saves the JSON
report as an artifact. See the [runtime guide](../../docs/RUNTIME_DATA.md) for the
implemented API and deployment limitations.

Use `--binary PATH` to select another executable and `--report PATH` to change
the JSON report location. The default report is
`target/runtime-data-acceptance.json`. The script creates temporary database,
credential, and TLS material, starts a local HTTPS JSON snapshot fixture, and
launches the server as a subprocess. It sends requests only to the server's
public HTTP API. Temporary state and generated credentials are removed when the
run ends. The report records each completed check and the failure text; the
process exits nonzero if any check fails.

The suite checks bearer authentication and exact scope grants, strict schemas
for scalars, arrays, nested records, and record-list aggregates, malformed JSON
and value rejection, atomic commits, missing and stale evidence, source replay
and expected-revision conflicts, failed pull retention, complete snapshot
replacement, and fact recovery after a server restart. The HTTPS fixture uses a
fresh CA and a certificate with `127.0.0.1` and `localhost` subject names, so
connector TLS verification is exercised without an external service.
