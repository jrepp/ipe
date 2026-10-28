function step(context, responses, state) {
  return {
    requests: [{
      id: "invalid",
      operation: "metadata",
      params: { sysparm_query: "name=task^ORactive=true" },
    }],
    state: null,
  };
}
