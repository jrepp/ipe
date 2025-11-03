# IPE Agent Workflows

Specialized agents for common Rust development workflows in the IPE project.

## Available Agents

### Development Workflows

#### [fix-bug.md](./fix-bug.md)
Make targeted bug fixes with surgical precision.
- Minimal changes
- Test-backed fixes
- Coverage maintained or improved
- One bug per PR

**Use when**: Fixing bugs, resolving errors, correcting issues

#### [implement-feature.md](./implement-feature.md)
Implement features with tests and clear PRs.
- Test-driven development
- Small, focused PRs (<400 lines)
- Clear why/what/how documentation
- Measurable quality

**Use when**: Adding features, building functionality, implementing requirements

#### [optimize-performance.md](./optimize-performance.md)
Make systems measurably faster using data-driven optimization.
- Profile-guided optimization
- Baseline benchmarks with criterion
- Measured improvements
- Documented trade-offs

**Use when**: Improving performance, reducing latency, optimizing algorithms

#### [tdd-workflow.md](./tdd-workflow.md)
Implement features using strict TDD methodology.
- Red-Green-Refactor cycle
- High coverage (>90%)
- Quality checks
- Test-first approach

**Use when**: Building new modules, implementing complex logic, ensuring quality

## How to Use

### In AI Assistant Context

Reference an agent in your prompt:
```
@workspace Follow the fix-bug workflow to resolve the stack overflow in nested expressions
```

### General Workflow

1. **Choose agent** based on task type
2. **Follow workflow** in agent guide
3. **Use principles** to guide decisions
4. **Create PRs** with required documentation
5. **Verify quality** before completing

## Project Integration

These agents align with IPE's core principles:

### Commit Standards
- Conventional commits: `type(scope): description`
- No branding or marketing
- Clear, concise messages
- Small, frequent commits

### PR Requirements
- **Why**: User intent and motivation
- **What**: What changed (facts)
- **How**: Implementation approach
- **Testing**: Verification steps
- **Performance**: Impact on metrics (if applicable)

### Quality Standards
- Tests for all changes
- Coverage maintained/improved (target: 90%+)
- All tests passing
- No clippy warnings
- Code formatted with rustfmt
- Benchmarks run for performance-critical code

## Agent Selection Guide

### Development Tasks

| Task Type | Agent | Branch Prefix |
|-----------|-------|---------------|
| Fix bug | fix-bug | `fix/` |
| Add feature | implement-feature | `feat/` |
| Optimize code | optimize-performance | `perf/` |
| Refactor with TDD | tdd-workflow | `refactor/` |

## IPE-Specific Considerations

### Performance-Critical Code
- Always benchmark with `cargo bench`
- Profile with `cargo flamegraph` or `perf`
- Target: p99 < 10µs (JIT), p99 < 50µs (interpreter)
- Throughput: 100K+ evals/sec/core

### Safety and Security
- No unsafe code without justification
- Pass all sanitizers (address, leak, thread)
- Supply chain audits (cargo-deny, cargo-audit, cargo-geiger)
- Memory-safe by default

### Concurrency
- Lock-free data structures (Arc, atomics)
- Immutable snapshots for concurrent reads
- Test with concurrent workloads
- Profile for contention

## Extending Agents

To add new agents:
1. Create `new-agent.md` in `.github/agents/`
2. Follow existing agent structure
3. Include clear workflow and principles
4. Add Rust-specific examples
5. Update this README

## Best Practices

- **One agent per task**: Don't mix workflows
- **Follow the workflow**: Agents provide proven patterns
- **Document everything**: PRs need why/what/how/testing/performance
- **Measure quality**: Use actual numbers, not opinions
- **Keep PRs small**: Easier to review and merge (<400 lines)
- **Test everything**: No untested code
- **Benchmark performance code**: Verify no regressions
- **Security first**: Run supply chain checks

## Related Documentation

- Project agent instructions: `/.agent/core-prompt.md`
- Architecture documentation: `/docs/ARCHITECTURE.md`
- Requirements specification: `/REQUIREMENTS.md`
- Bytecode specification: `/docs/BYTECODE.md`
- Commit message standards: See `.agent/core-prompt.md`
- Git workflow best practices: See `.agent/core-prompt.md`
