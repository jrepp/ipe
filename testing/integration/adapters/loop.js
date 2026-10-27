function step(context, responses, state) {
  if (responses.length === 0) {
    return {
      requests: [{ id: "catalog", operation: "metadata", params: {} }],
      state: null,
    };
  }
  if (responses[0].code === "provider_unavailable") {
    while (true) {}
  }
  return { outputs: { recovered: true }, state: null };
}
