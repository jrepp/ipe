# Writing and UX terms

Read this when changing UX text, documentation, or reports.

## Writing

Use [ASD-STE100, Issue 9](https://www.asd-ste100.org/STE_downloads.html) for UX text, documentation, and agent reports.

- Use active voice and familiar words. State who does what.
- Keep instructions to 20 words per sentence and descriptions to 25.
- Give one instruction per step. Put steps in order.
- Keep each paragraph on one topic, with no more than six sentences.
- Use one term for each meaning. Define technical terms and keep identifiers unchanged.
- Avoid jargon, promotional language, vague labels, and long noun groups. Keep words needed for clarity.
- Name actions in buttons. State the problem and next action in errors.
- Distinguish measured values, source reports, unknown values, and sample data.

These rules summarize the standard. Check its full rules and dictionary before claiming compliance.

### UX terms

- **Policy:** A rule evaluated by an enforcement node.
- **External policy:** A requirement enforced by an external system and described by an integration.
- **VPC:** Virtual private cloud.
- **Policy service layer:** Services that store and distribute policy data within a VPC. This is a design term, not proof of implementation.
- **Cluster:** Nodes with the same service role.
- **Enforcement node:** A node that evaluates requests against assigned policies.
- **Integration node:** A node that reads external systems and publishes permitted data. Check the current workload's supported operations.
- **Assigned revision:** The policy revision a node must use.
- **Active revision:** The policy revision a node reports as loaded.
- **Source observation:** Data read from an external system at a stated time.
