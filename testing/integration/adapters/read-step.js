// Runs in the embedded JSON step guest. It receives no Node or host APIs.
function step(context, responses, state) {
  if (responses.length === 0) {
    return {
      requests: [{ id: "catalog", operation: "metadata", params: {} }],
      state: { stage: "requested" },
    };
  }

  return {
    outputs: {
      connection_ref: context.connection_ref,
      records: responses[0].data,
      source_receipt: responses[0].receipt,
      node_api_absent: typeof process === "undefined",
      fetch_absent: typeof fetch === "undefined",
      origin_absent: context.origin === undefined,
      credential_absent: context.credential === undefined,
      prior_state: state.stage,
    },
    state: null,
  };
}
