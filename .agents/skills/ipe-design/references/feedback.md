# Design feedback

This record captures feedback from the IPE preview work. Apply it within this product context.

| User feedback | Design lesson | Status |
| --- | --- | --- |
| “The meanings and connections in the data are still not clear.” | Explain the policy, control, evidence, source, and release relationships. IDs alone do not explain them. | User requirement |
| “I like the policy + control frame.” | Use one compact chain with controls as a checklist. | User preference |
| “Give evidence more screen real estate and flexibility.” | Make evidence the main detail workspace. Let users expand it and inspect linked objects. | User preference |
| “This new view works much better.” | Retain the compact checklist and broad evidence workspace. | User-approved direction |
| ServiceNow records, owners, email, and Slack profiles must appear together. | Link source records and responsible people to each control. Preserve source identity and record state. | User requirement |
| “How the policy is being used, what it's optimizing, gating and costing.” | Explain use, decisions, findings, tokens, and timing at chain level. | User requirement |
| “Rewrite the preview language using ~/d/SimpleEnglish.” | Use short, complete sentences and consistent terms. Preserve domain meaning. | User requirement |
| “I don't like this kind of phrasing: What it permits, What it can reduce.” | Use direct content labels. Simple words alone do not make a useful heading. | Rejected pattern |
| “The usage report is just sitting on top of the policy chain.” | Integrate report navigation with the policy that owns it. Do not place a large report above the chain. | Rejected placement |

The current revision puts **Usage report** under the central policy and opens it in the detail workspace.
This is an implementation for review, not a user-approved layout yet.

## Copy examples

| Avoid | Prefer | Reason |
| --- | --- | --- |
| What it permits | Release decision | Names the result |
| What it can reduce | Token and time comparison | Names the measured quantities |
| What it found | Findings | Names the content |
| Summed work | Total work time | Names the quantity in familiar words |
| Raw-source baseline | Direct source reads | Names the comparison path |
| Illustrative simulation | Sample data | Marks the data clearly |

These examples are local choices. Keep a technical term when a shorter label would change its meaning.
