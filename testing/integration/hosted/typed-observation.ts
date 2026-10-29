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
