/// <reference path="../../../crates/ipe-integration/sdk/index.d.ts" />
// This fixture checks the authored observation API without exposing native credentials.
function typedObservation(receipt: string): Ipe.Observation {
  return ipe.observation({
    schema_version: 1,
    scope: 'workflow:example/repo/docs',
    claim: 'provider_reported',
    entity: {
      kind: 'fact', subject: 'deployment:42', predicate: 'status', value: 'success',
      authority: 'example', execution: null,
    },
  }, {provider_status: 'success'}, [receipt]);
}
void typedObservation;

// A resumable plan stores JSON only. Native reads remain outside the guest.
function typedPlan(context: Ipe.HostedContext, responses: Ipe.Response[], state: Ipe.TaskState | null): Ipe.StepResult {
  const plan = ipe.tasks(state);
  if (state === null) {
    plan.enqueue({kind: 'sample'}, 'sample');
    const task = plan.next();
    return ipe.exchange([plan.request(task, context.operations[0], {})], plan.state, [], plan.progress('dispatch', 'sample'));
  }
  plan.receive(responses);
  return ipe.complete({coverage: 'sample'}, plan.state);
}
void typedPlan;
