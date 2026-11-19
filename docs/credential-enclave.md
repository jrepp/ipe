# Credential enclave for sandbox cells

Status: Proposal. This document is a design. Nothing here is implemented.

Related: meridian-gantry `docs/dev/sandbox-sessions-design.md` (section
D2.1), t1-hosting `docs-cms/rfcs/rfc-014`.

## Problem

Sandbox cells do work for a project on behalf of a user. The work needs the
user's credentials: a push token for git, a signing key for commits, later
tokens for other services. Today an operator copies credential files into a
cell by hand. That puts long-lived secrets inside the cell boundary, does
not scale, and leaves no audit trail.

## Goal

A dedicated credential enclave service. The operator populates the enclave
once. Cells ask the enclave for credentials at the moment of use. The
enclave answers with short-lived, scope-limited credentials under policy.

## How a cell proves who it is

The enclave must know three facts about a caller before it issues anything:

1. Which sandbox is asking.
2. What that sandbox was spawned to do: project, template, role.
3. That the spawn binding is genuine.

The design uses the sandbox identity the gateway already issues. A cell
presents its sandbox credential with each request. The enclave checks that
credential against the gateway, then checks its own policy:

```text
project = t1-hosting AND template = generalist AND state = running
  -> allow git-push (ttl 900s), git-sign (ttl 3600s)
```

The spawn binding comes from Gantry. When the pool control spawns a cell,
the record names the project and template. The enclave trusts that record,
not a claim from the cell. A cell cannot widen its own scope: it can only
ask for credentials its spawn binding allows.

## Issuance model

- Credentials are short-lived. Push tokens live minutes, not days. Signing
  keys are per-cell and expire with the cell.
- Each issue event is a record: caller identity, credential kind, scope,
  time to live, time. The record is the audit trail.
- Rotation is automatic: a helper asks again near expiry and gets a fresh
  credential. Revocation is enclave-side: delete the policy or the grant,
  and later requests fail. Issued credentials expire on their own.
- The enclave never sends a long-lived secret to a cell. The operator's
  signing key and base tokens stay in the enclave.

## First credential set: git

Two credentials cover most cell work:

1. **Push token.** A short-lived token with write access to the project's
   repositories. The cell uses it as a git credential for push and pull.
2. **Signing key.** An ephemeral SSH signing key. On first issue for a
   cell, the enclave registers the public key with the project as an
   approved signer and removes it when the cell stops. Commits then verify
   with `git commit -S` and no long-term key exists.

## In-cell tools

The cell gets two small tools, installed with the cell template:

- A git credential helper. Git calls it for push and pull. The helper asks
  the enclave and caches the answer for the token lifetime.
- A signing wrapper. `git commit -S` calls it. The wrapper asks the
  enclave for the signing key material, signs, and drops the key material
  after the call.

The tools talk only to the enclave endpoint. The cell policy allows that
one destination. Credentials exist in cell memory for the duration of a
call, not in cell files.

## Mapping to IPE

| Enclave concept | IPE primitive |
| --- | --- |
| Issuance rule | Typed policy (language 2) |
| Issued credential | Runtime data fact with mandatory TTL |
| Issue and revoke events | Scoped authenticated fact writes |
| Helper protocol | HTTPS surface (pull) |
| Policy inspection and audits | CLI and MCP access |
| Deployment | Deployment guide, single service |

The enclave is an IPE-backed service: a small API over the existing
runtime data store. No new storage, no new policy language.

## Network placement

The enclave runs on the private network, reachable from cell hosts only.
Two placements fit current deployments:

1. Beside the sandbox gateway on the cell host. Cells reach it over the
   gateway's loopback or host network.
2. On the tailnet, behind the same private gateway that fronts other
   services. Cell policies name the enclave endpoint.

Placement 1 keeps credential traffic off the network. Placement 2 works
when one enclave serves several cell hosts. Both use the gateway's
certificate.

## Open questions

1. Which component signs the sandbox credential the enclave checks: the
   sandbox gateway, or the pool control that spawned the cell?
2. Who approves a new ephemeral signing key for a project: an operator, or
   the enclave under its own policy?
3. How do helper tools reach a project repository list for token scoping:
   a policy field, or a Gantry query at request time?
4. Do we need a renewal push (enclave tells the helper before expiry), or
   is pull-on-use enough?

## First slice

1. IPE policy that allows `issue-credential` writes for one project and
   one credential kind.
2. An HTTPS endpoint that checks a sandbox credential, evaluates the
   policy, and returns a short-lived push token.
3. A git credential helper that uses that endpoint.
4. One cell, one project, push verified end to end.
