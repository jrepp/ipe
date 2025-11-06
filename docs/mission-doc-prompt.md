# Reusable Prompt: Create Comprehensive Mission Documentation

Use this prompt to generate technical mission documentation for any repository.

---

## Prompt: Create Comprehensive Mission Documentation for Technical Repository

**Context:** I have a technical repository that needs comprehensive mission documentation for technical leaders and teams taking over the project.

**Requirements:**

### 1. Mission Statement
- Write for technical leaders, not marketing
- Be pragmatic and realistic about current state
- Be future-forward with concrete milestones
- Emphasize key technical insights (performance, security, operational concerns)
- Avoid superlatives and hype language
- Include specific, measurable objectives

### 2. Core Content Sections

**Problem Statement:**
- Current state of the problem space
- Specific pain points with existing solutions
- What's needed (with measurable targets)
- This project's technical approach

**Design Principles:**
- Why technology choices matter (e.g., language selection reasoning)
- Acknowledge trade-offs honestly
- Explain architectural decisions with rationale
- Focus on production operational concerns

**Architecture Deep Dive:**
Include:
- Policy data delivery and how it enables high availability
- Predictability of the system (latency, throughput)
- Separation of concerns between components
- Data layer architecture (e.g., RocksDB) and how it enables flexibility
- Multi-tenancy and isolation mechanisms
- Various deployment models and use cases

**Use Cases:**
- Provide 6-8 detailed use cases with code examples
- Show separation from application data model
- Demonstrate flexible entity modeling
- Include examples like: authorization, feature flags, workflows, compliance, multi-tenancy, etc.

### 3. Technology Justification

**If using Rust (or similar systems language):**
- Explain why memory safety matters for this component
- Compare with alternatives (Go, Java, C++)
- Discuss performance predictability (no GC pauses)
- Address security implications (CVE prevention)
- Acknowledge learning curve trade-offs

**For any infrastructure component:**
- Emphasize line-rate performance requirements
- Explain why it needs to run on the critical path
- Discuss operational reliability needs

### 4. Component Status & Ownership

Create tables showing:
- All major components with completion percentage
- Test coverage per component
- Production readiness assessment
- File locations for each component
- Recommended team ownership

### 5. Large Team Initiatives

Outline 5-6 major initiatives for a team taking over, including:
- Duration estimates (realistic)
- Team size recommendations
- Specific objectives
- Success metrics (measurable)

### 6. Current Status Section

**Be honest about:**
- What's production-ready today
- What's not ready (with specifics)
- Recommended adoption path (phased approach)
- Technical debt and risks
- Known gaps
- Operational concerns
- Mitigation strategies
- Success criteria (measurable)

### 7. Cross-Language/Tool Compatibility

If applicable, analyze:
- Can competing tools' formats compile to your format?
- Compatibility percentage estimates
- Migration strategies with code examples
- Challenges and limitations
- Recommendation for implementation priority

### 8. References Section

Create comprehensive tables for:
- Core documentation (with line counts)
- Technical specifications
- RFCs/design documents
- Test results (total, by category, with coverage)
- Code quality metrics (coverage, linting, security)
- Performance benchmarks (targets vs. actuals)
- Implementation status by component
- CI/CD status
- Key achievements with dates
- Technology stack with versions
- Performance targets vs. actuals
- Repository statistics

### 9. Formatting Requirements

- Remove any AI assistant references
- Use markdown tables extensively
- Include code examples in appropriate language
- Use mermaid diagrams where helpful
- Keep tone professional and technical
- Avoid marketing language
- Use specific numbers instead of superlatives
- **No emojis** - use text markers like [DONE], [WIP], [PLANNED], [NO]

### 10. Key Insights to Highlight

For your specific domain, emphasize:
- [Your key technical insight here, e.g., "line-rate performance"]
- [Your architectural pattern here, e.g., "separation of policy from data"]
- [Your reliability approach here, e.g., "zero-downtime updates"]

### 11. Language and Structure

- **Simplify language:** Use clear, direct sentences
- **Good narrative structure:** Problem → Solution → Architecture → Implementation → Status
- **Avoid jargon overload:** Explain technical terms when first used
- **Short paragraphs:** 2-4 sentences maximum
- **Active voice:** "The system does X" not "X is done by the system"

**Output:** Generate a comprehensive MISSION.md file (1000-1500 lines) suitable for:
- Technical leaders evaluating the project
- Teams taking over development
- Architects planning integration
- Operations teams deploying and maintaining

The document should be realistic, technical, and actionable rather than aspirational or marketing-focused.

---

## Usage Instructions:

1. **Preparation:**
   - Gather key repository information (tech stack, architecture, current status)
   - Identify main problems the project solves
   - List key technical decisions and trade-offs

2. **Customization:**
   - Replace bracketed sections with project-specific details
   - Add repository context (what it does, why it exists)
   - Specify key technical insights (performance targets, security requirements)
   - Identify use cases relevant to your domain

3. **Execution:**
   - Request codebase exploration first if needed
   - Generate initial draft
   - Iterate on sections needing more/less detail
   - Review for technical accuracy and clarity

4. **Refinement:**
   - Remove marketing language and superlatives
   - Simplify complex sentences
   - Add code examples where helpful
   - Ensure narrative flows logically
   - Verify all numbers and metrics are accurate

## Example Customization:

For an authorization engine project:
```
Key Insights:
- Line-rate performance: <100μs latency enables inline checks
- Separation of concerns: Policy logic separate from application data
- Zero-downtime updates: Atomic snapshot swaps with no blocking

Key Problems:
- Existing solutions have unpredictable GC pauses (2ms+ spikes)
- Policy updates require service restarts
- Difficult to integrate approval workflows

Technical Approach:
- Rust for memory safety and predictable performance
- Bytecode compilation with optional JIT
- Lock-free concurrent reads via immutable snapshots
- RocksDB for authorization context (approvals, relationships)
```

## Document Structure:

The generated document should follow this narrative:

1. **Mission Statement** - What and why in one sentence
2. **Core Objectives** - 6 specific goals
3. **Design Principles** - How decisions are made
4. **Problem Statement** - What's broken, what's needed
5. **What We're Building** - Phases and timeline
6. **Component Status** - Detailed inventory
7. **Large Initiatives** - Work for incoming teams
8. **Why This Approach** - Technical justification
9. **Architecture** - How it works (data flow, HA, separation)
10. **Use Cases** - Real-world examples
11. **Current Status** - Honest assessment
12. **References** - Comprehensive tables

This creates a logical flow: Problem → Solution → Implementation → Status → Next Steps
