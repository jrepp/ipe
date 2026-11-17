# Temporary MCP SDK patch

Source: published [rmcp 3.11.0](https://crates.io/crates/rmcp/3.11.0), Apache-2.0.
The original LICENSE remains unchanged. Package archive SHA-256: `391f153431ab68664d946513ea0cb3edbca78eaf99c5a983036cacab6220d85b`.

[RUSTSEC-2026-0543](https://github.com/RustSec/advisory-db/blob/main/crates/chrono/RUSTSEC-2026-0543.md) marks every Chrono version unmaintained.
The published SDK requires Chrono even with default features disabled.
[IPE-CHANGES.patch](IPE-CHANGES.patch) records the complete code and manifest delta from the registry source.
Registry cache markers and the unused upstream Cargo.lock are omitted. Cargo.toml.orig remains the original source manifest.

Replace timestamp operations with Jiff 1.0. Keep timestamp wire values as RFC 3339 strings and epoch milliseconds.
The Rust annotation builder accepts jiff::Timestamp instead of chrono::DateTime. IPE does not use that builder API.
Timeout diagnostic text uses std::time::Duration debug formatting. Error variants and timeout behavior remain unchanged.
Remove Chrono schema features because annotation timestamps are stored as strings.
One upstream test comment loses trailing spaces to pass repository whitespace checks.
Protocol, transport, authorization, framing, and execution limits are unchanged.

P2 owns this patch. Remove it when a published official SDK removes Chrono and passes the same protocol and timestamp checks.
Do not add SDK features or unrelated fixes here. Recheck the exact diff and license on each version update.
The Cargo patch is workspace-wide, so the server and independent test client use the same SDK source.
The crate stays outside workspace membership; run its focused library tests through its manifest when changing this patch.
