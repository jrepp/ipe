# IPE RFCs (Request for Comments)

**Design documents and technical specifications for IPE features**

## What are RFCs?

RFCs (Request for Comments) are design documents that describe major features, architectural decisions, and technical specifications for the Idempotent Predicate Engine. Each RFC goes through a review process before implementation.

## RFC Process

```
Draft → Review → Accepted → Implemented → Final
```

- **Draft:** Initial proposal, open for feedback
- **Review:** Under active discussion
- **Accepted:** Approved for implementation
- **Implemented:** Feature completed
- **Final:** Documentation updated, RFC archived

## Active RFCs

### Core Engine (Original Architecture)

The original RFC.md has been superseded by focused RFCs below. See [archive/RFC.md](../docs/archive/) for historical reference.

### Sidecar Service (New Architecture)

| RFC | Title | Status | Summary |
|-----|-------|--------|---------|
| [001](001-sidecar-service-architecture.md) | Sidecar Service Architecture | Draft | Minimal Rust service with data/control plane separation, <50MB footprint |
| [002](002-sse-json-protocol.md) | SSE/JSON Protocol | Draft | MCP-inspired protocol using Server-Sent Events and JSON-RPC |
| [003](003-policy-tree-storage.md) | Policy Tree Storage | Draft | Content-addressable tree structure with cryptographic hashing |

## Planned RFCs

| Number | Title | Target | Description |
|--------|-------|--------|-------------|
| 004 | Data Storage | TBD | Dynamic data store for approvals and external signals |
| 005 | Client Libraries | TBD | Official clients for Rust, Python, Node.js, Go |
| 006 | Observability | TBD | Metrics, tracing, and monitoring specification |
| 007 | Security Model | TBD | Authentication, authorization, and audit logging |

## RFC Guidelines

### When to Write an RFC

Write an RFC for:

- Major new features or subsystems
- Significant architectural changes
- Public API changes
- Breaking changes to existing behavior
- Complex implementation requiring design review

**Don't** write an RFC for:

- Bug fixes
- Minor improvements
- Internal refactoring
- Documentation updates
- Performance optimizations (unless requiring architectural changes)

### RFC Template

```markdown
# RFC-XXX: Title

**Status:** Draft | Review | Accepted | Implemented | Final
**Created:** YYYY-MM-DD
**Author:** Name
**Depends On:** RFC-XXX (optional)

## Summary
2-3 sentence overview of the proposal.

## Motivation
Why is this needed? What problem does it solve?

## Design Goals
3-5 bullet points of what the design should achieve.

## Detailed Design
The meat of the RFC. Include:
- Architecture diagrams
- API examples
- Data structures
- Algorithms
- Trade-offs

## Implementation Phases
Break down into manageable milestones.

## Alternatives Considered
What other approaches were evaluated and why were they rejected?

## Success Metrics
How will we measure if this is successful?

## References
Links to relevant documentation, papers, prior art.
```

### Writing Tips

1. **Start with motivation** - Make the "why" crystal clear
2. **Use diagrams** - Architecture diagrams, sequence diagrams, flow charts
3. **Show code examples** - Demonstrate the API or usage patterns
4. **Address trade-offs** - No design is perfect, be honest about limitations
5. **Be concise** - Aim for 2000-3000 words for most RFCs
6. **Include metrics** - Define success criteria upfront

## RFC Review Process

1. **Create PR** - Submit RFC as a PR with `[RFC]` prefix
2. **Discussion** - Team reviews and provides feedback (1-2 weeks)
3. **Revisions** - Author addresses feedback
4. **Decision** - Accept, reject, or request more changes
5. **Merge** - Accepted RFCs are merged with status updated

## Reading Guide

**New to IPE?**
1. Start with [SUMMARY.md](../SUMMARY.md) for the vision
2. Read [ARCHITECTURE.md](../docs/ARCHITECTURE.md) for the original design
3. Review RFCs 001-003 for the sidecar architecture

**Implementing a feature?**
1. Find relevant RFC in the table above
2. Read "Detailed Design" section
3. Check "Implementation Phases" for milestones
4. Reference during development

**Want to contribute?**
1. Review [CONTRIBUTING.md](../CONTRIBUTING.md)
2. Check "Planned RFCs" for opportunities
3. Propose new RFCs via GitHub Discussion first

## RFC Status Legend

| Status | Meaning |
|--------|---------|
| 📝 Draft | Initial proposal, actively being written |
| 👀 Review | Under team review, seeking feedback |
| ✅ Accepted | Approved, ready for implementation |
| 🚧 Implementing | Feature in active development |
| ✔️ Implemented | Code complete, testing in progress |
| 📚 Final | Shipped and documented |
| ❌ Rejected | Proposal declined |
| 🔄 Superseded | Replaced by newer RFC |

## Quick Links

- **[Documentation Index](../docs/INDEX.md)** - All IPE documentation
- **[Architecture](../docs/ARCHITECTURE.md)** - System architecture overview
- **[Contributing](../CONTRIBUTING.md)** - How to contribute
- **[GitHub Issues](https://github.com/jrepp/ipe/issues)** - Bug reports and feature requests

---

**Last updated:** 2025-10-27
