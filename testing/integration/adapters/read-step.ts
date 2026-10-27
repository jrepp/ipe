/// <reference path="../../../crates/ipe-integration/sdk/index.d.ts" />

function step(context: Ipe.Context, responses: Ipe.Response[], state: Ipe.Json): Ipe.StepResult {
  if (state === null) {
    return ipe.requests([ipe.request("catalog", "metadata")], { stage: "requested" });
  }
  const records = ipe.records(responses, "catalog");
  const result = ipe.response(responses, "catalog");
  return ipe.complete({
    connection_ref: context.connection_ref,
    records,
    source_receipt: result.receipt ?? null,
    node_api_absent: typeof process === "undefined",
    fetch_absent: typeof fetch === "undefined",
    origin_absent: !("origin" in context),
    credential_absent: !("credential" in context),
    prior_state: typeof state === "object" && !Array.isArray(state) ? state.stage : null,
    sdk_version: ipe.version,
  });
}
