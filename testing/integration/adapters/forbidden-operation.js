function step(context, responses, state) {
  return {
    requests: [{ id: "forbidden", operation: "arbitrary_route", params: {} }],
    state: null,
  };
}
