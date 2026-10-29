# Writing rule: ASD-STE100

Use ASD-STE100 Simplified Technical English for new or revised user-facing text. This includes UX copy, documentation, and agent reports.

Use [ASD-STE100, Issue 9](https://www.asd-ste100.org/STE_downloads.html) as the writing reference. Its writing rules and dictionary are authoritative. The following project rules are a practical checklist, not the complete standard.

- Write direct sentences. Give each sentence one main subject.
- Use active voice. Name the person or system that performs the action.
- Use a maximum of 20 words in procedural sentences and 25 words in descriptive sentences.
- Give one instruction in each step. Put steps in the required order.
- Use one term for one meaning. Define necessary technical terms before use.
- Use approved words where possible. Treat required product names and domain terms as technical names.
- Keep each paragraph on one topic. Use no more than six sentences in a paragraph.
- Use short noun groups. Replace long noun groups with phrases.
- Keep articles and other necessary words. Do not shorten text until its meaning becomes unclear.
- Do not use idioms, promotional language, or vague status labels.
- Write button labels as clear actions. State the problem and the next action in error messages.
- Distinguish measured values, source reports, unknown values, and sample data.
- Review text against the standard before claiming full ASD-STE100 compliance. A sentence-length check alone cannot establish compliance.

## UX terms

- **Policy:** A rule that an enforcement node evaluates.
- **External policy:** A requirement that an external system enforces. An integration reads its description.
- **Policy service layer:** The services that store and distribute policy data within a VPC.
- **VPC:** A virtual private cloud. Define this term in new introductory material.
- **Cluster:** A group of nodes that perform the same service role.
- **Enforcement node:** A node that evaluates requests against assigned policies.
- **Integration node:** A node that reads external systems and publishes permitted data.
- **Assigned revision:** The policy revision that a node must use.
- **Active revision:** The policy revision that a node reports as loaded.
- **Source observation:** Data that an integration read from an external system at a stated time.

Use identifiers and API field names unchanged. Explain their meaning in the surrounding text when necessary.
