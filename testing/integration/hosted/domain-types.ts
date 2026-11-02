/// <reference path="../../../crates/ipe-integration/sdk/index.d.ts" />

// The operation API projects v1 records. Collectors continue to emit the v1 envelope.
function domainTypes(operation: Ipe.OperationDescription, receipt: string): Ipe.Observation {
  const identity: Ipe.OperationId = operation.operation;
  const legacyIdentity: Ipe.ControlId = identity;
  const named: Ipe.NamedRequirement = operation.requirements[0];
  const specification: Ipe.RequirementSpecification = named.specification;
  const legacySpecification: Ipe.Requirement = specification;
  const legacyItem: Ipe.Obligation = {
    id: named.id,
    description: named.description,
    requirement: legacySpecification,
    next_observation: named.next_observation,
  };
  const request: Ipe.AssessmentRequestV2 = {
    operation: identity,
    source_revision: operation.source.revision,
    record_revision: operation.record_revision,
    context: {artifact: 'sha256:example'},
  };
  const response: Ipe.OperationsResponse = {
    schema: 'ipe.integration.operations.v2',
    accepted_revision: null,
    candidate_revision: null,
    operations: [operation],
  };
  void request;
  void response;
  void ipe.operationKey(identity);
  void ipe.controlKey(legacyIdentity);
  return ipe.observation({
    schema_version: 1,
    scope: operation.scope,
    claim: operation.claim,
    entity: {
      kind: 'constraint', control: legacyIdentity, source: operation.source,
      title: operation.title, actor_context: operation.actor_context,
      applicability: operation.applicability, obligations: [legacyItem],
      dependencies: operation.dependencies, enforcer: operation.declared_enforcer,
    },
  }, null, [receipt]);
}
void domainTypes;

function assessTypes(result: Ipe.AssessmentResponseV2): void {
  const check: Ipe.EvidenceCheckV2 = result.requirements[0].evidence_check;
  const fulfillment: 'unknown' = result.requirements[0].fulfillment.status;
  const authorized: false = result.authorization;
  void check;
  void fulfillment;
  void authorized;
  // @ts-expect-error Evidence matches do not establish authorization.
  const allow: true = result.authorization;
  void allow;
}
void assessTypes;
