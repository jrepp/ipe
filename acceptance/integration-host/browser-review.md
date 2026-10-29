# Browser review, 2026-10-29

Agent-browser reviewed the running local host in an isolated Chromium session.
The review used browser behavior, not source inspection. The server preceded the current outcome archive change.

| Severity | Finding |
| --- | --- |
| High | Automatic refresh closed open source and evidence disclosures within three seconds |
| Medium | Signed-out entry named the launcher without a location or launch command |
| Medium | Failed extraction rows showed `script_failed` without inspectable diagnostics |
| Medium | External outcome summaries omitted execution date, commit, and observation time |
| Medium | At 390 pixels wide, document content extended to 679 pixels |
| Low | Axe found a missing main landmark and content outside landmarks |

The private launcher worked. Manual run 81 succeeded in 8574 ms, with 22 records and three outcomes visible.
No JavaScript exceptions appeared. Video capture could not run because ffmpeg was absent.
The detailed local report and screenshots are in `target/host-browser-review/`.
The disclosure issue now has a regression test for unchanged and changed data, including keyboard focus.
The remaining findings inform the active six-part model implementation.

After the fix, agent-browser reopened the live installed source and waited four seconds. The disclosure stayed open.
The live authenticated outcome export returned three execution identities with one payload change each.
