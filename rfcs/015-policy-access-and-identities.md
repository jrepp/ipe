# RFC 015 Policy access and opaque identities

**Status:** Draft discovery
**Created:** 2026-10-18
**Parent and MVP:** [RFC 010](010-remediation-adapters-and-external-policy-discovery.md)

## Scope

Own permitted information release, opaque identities, constraint/action disclosure,
and elevation requirements. It applies to every API, tool, report and trace. Audit
retention and reconstruction belong to [RFC 016](016-integration-signals-and-audit.md).
[RFC 017](017-enforcement-mcp-surface.md#7-principal-identity-request-control-and-finalization)
defines the proposed enforcement principal/API profiles and revocation gates. Its
[trace projection](017-enforcement-mcp-surface.md#6-trace-projection-and-diagnostic-retention)
applies this contract to event structure, predicate results and coverage metadata;
its hosted profile prevents transport replay from bypassing current authority.

## Access contract

**Need-to-Know Policy Access** governs disclosure; **Just-in-Time Privileged Access**
governs temporary elevation. These names use established
[need-to-know](https://csrc.nist.gov/glossary/term/need_to_know),
[least-privilege](https://csrc.nist.gov/glossary/term/least_privilege), and
[temporary privilege activation](https://learn.microsoft.com/en-us/entra/id-governance/privileged-identity-management/pim-configure)
concepts. The combined capability name is a project term, not a certification.
Emergency break-glass access is a separately governed exception.

### Opaque references and permitted fields

The host allocates stable random tenant-scoped identities. A restricted integration
vault maps them to physical endpoints, provider resource identities, and historical
mapping versions. Endpoint or credential rotation does not change resource identity.
Do not derive public IDs from guessable URLs or expose globally correlatable aliases.
An ID is not a capability; resolution requires separate authorization.

Enforcement-facing data and ordinary policy descriptions contain no source URLs,
hostnames, internal tenant names, repository paths, connection strings, or secrets.
The platform controls field allowlists and classifications; adapters cannot
approve their own disclosure. Host-controlled receipts record what was fetched
and when; source observation timestamps require validation rather than replacement
with receipt time. Keep original payloads, source digests, and detailed diagnostics
restricted. Integrity digests for delivery cover sanitized envelopes.

Apply these controls to nested values, descriptions, errors, logs, metrics labels,
traces, fixtures, and derived indexes. Public errors carry stable codes and opaque
diagnostic IDs. Approved workflow tooling uses the broker to operate on an opaque
reference without receiving connection secrets. Policy reading, identity resolution,
connection use, and action execution are separate privileges.

A sandbox and a schema cannot prove semantic truth or prevent every covert channel.
Minimize guest inputs and granted destinations, and test mappings independently.
Useful facts may still permit inference; opaque identity removes explicit topology
references, not all information flow.

### Disclosure and elevation

| Caller-relative state | Permitted response |
| --- | --- |
| Visible | Authorized description with opaque references |
| Masked | Safe summary; protected fields withheld |
| Elevation required | Safe notice and permitted elevation requirements |
| Unavailable | Safe notice without a promise that elevation can obtain details |

The platform computes these states. Where existence is itself protected, omit
individual entries, counts, and distinguishing errors; expose only any permitted
aggregate indication of incomplete knowledge. Distinguish disclosure unavailability
from an unavailable source observation. The same rules cover enforcement action
names, targets, reasons, and evidence.

Elevation requirements specify eligible identity or role, resource/field scope,
purpose, justification or case reference, authentication assurance, approval and
separation of duties, duration, allowed operations, and audit obligations. Show
unmet requirements only where authorized. Agents cannot approve themselves or
bypass human authentication requirements. Grants are scoped, audience-bound,
short-lived, revocable, and checked on access. Cached elevated views must expire
or invalidate; previously disclosed information cannot be recalled.

A separately administered access policy governs elevation with a bounded bootstrap
path, avoiding recursive requests to reveal the access policy itself. Its outage
denies new elevation/disclosure without blocking ordinary local evaluation.
Masking does not remove applicable rules. If enforcement lacks required evidence,
the result is indeterminate; if a caller cannot inspect a rule, the rule still applies.

## Area roadmap

| Phase | Deliverable | Proof |
| --- | --- | --- |
| M1 onboarding and discovery | Secret-isolated connection config, opaque refs, authenticated tools and allowlisted metadata | Seeded secrets/topology absent from responses/errors; unknown ref grants no access |
| M2 adapter iteration | Release schemas and masked fixtures | Agent output cannot authorize its own disclosure; stage results obey field grants |
| M3 first live integration | Caller-relative descriptions and explicit elevation requirements | Hidden rules still apply; access gaps remain gaps; existing locator leakage paths addressed |
| M4 operational proof | One scoped temporary inspection grant and expiry/revocation | No self-approval, topology resolution or execution privilege gained implicitly |
| Later | Enterprise identity/PAM integrations and separately governed break-glass | Same access contract across identity providers |

## Review questions

- Which schema identifiers and samples can the initial agent inspect?
- Which legacy provenance fields require migration before any live publication?
- How are temporary grants revoked and cached projections invalidated?
