/** Pure helpers provided by the host; transport and credentials remain native. */
declare namespace Ipe {
  type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
  interface Context {
    connection_ref: string;
    operations: { name: string; parameters: string[] }[];
    sdk_version: string;
  }
  interface Progress { phase: "dispatch"|"response"|"emit"|"complete"; task: string|null; queued:number; completed:number; gaps:number }
  type TaskState = {queue:{task:Json;key:string}[];seen:string[];sequence:number;current:{task:Json;id:string}|null;gaps:{task:Json;code:string}[];droppedGaps:number};
  interface TaskPlan {
    state: TaskState;
    enqueue(task:Json,key:string,depth?:number):boolean;
    next():Json;
    request(task:Json,operation:string,params:Record<string,string>):Request;
    receive(responses:Response[]):{task:Json;result:Response}|null;
    gap(task:Json,code:string):void;
    progress(phase:Progress['phase'],task?:string|null):Progress;
  }
  type ReadResult = Extract<Response, {code:string}> | (Extract<Response, {receipt:string}> & {observations:(scope:string,claim:Claim)=>{emit(entity:Entity,data?:Json):Observation}});
  interface HostedContext {
    integration_id: string;
    run_id: number;
    observed_at: number;
    input: Json;
    operations: string[];
    capabilities: {name:string;parameters:Record<string,{kind:string;values?:string[]}>;decoder:string}[];
    limits: {steps:number;observations:number;requests_per_step:number;observations_per_step:number};
    sdk_version: string;
  }
  interface Request { id: string; operation: string; params: Record<string, string> }
  type Response =
    | { id: string; operation: string; receipt: string; data: Record<string, Json>[]; source_digest?: string; truncated?: boolean; pagination?: string; snapshot_source?: Source | null; code?: never }
    | { id: string; operation: string; code: string; data?: never; receipt?: never };
  type StepResult = { requests: Request[]; state: Json; observations?: EmittedObservation[]; progress?: Progress } | { outputs: Json; state: Json };
  interface DefinitionId { namespace: string; resource: string; path: string }
  interface Source { definition: DefinitionId; revision: string; pointer: string }
  /** Identity of a declared external operation, not an operating enforcement mechanism. */
  interface OperationId { namespace: string; resource: string; operation: string; local_id: string }
  /** Compatibility name used by v1 records for an operation identity. */
  type ControlId = OperationId;
  interface ExecutionId { namespace: string; resource: string; run: string; attempt: number; job: string | null; step: number | null }
  type Claim = "declared" | "provider_reported" | "inferred" | "unknown";
  type Resolution = "resolved" | "candidate" | "unknown";
  interface Binding { name: string; origin: "caller" | "declared_default" | "absent" | "expression"; value: Json }
  type RequirementSpecification =
    | {kind:"fact"; query:{subject:string; predicate:string; expected:Json}}
    | {kind:"opaque"; reason:string}
    | {kind:"permission"; requested:Json}
    | {kind:"environment"; reference:Json}
    | {kind:"dependency"; control:ControlId};
  /** Compatibility name for the v1 requirement specification. */
  type Requirement = RequirementSpecification;
  /** A named requirement in the read-only operation API. It does not establish an actor's duty. */
  interface NamedRequirement { id:string; description:string; specification:RequirementSpecification; next_observation:{operation:string;reason:string}|null }
  /** V1 assessment item. This wrapper does not by itself establish a normative obligation. */
  interface Obligation { id:string; description:string; requirement:Requirement; next_observation:{operation:string;reason:string}|null }
  type Entity =
    | {kind:"definition"; source:Source; title:string; inputs:{name:string;required:boolean|null;default:Json}[]; outputs:string[]}
    | {kind:"evidence"; source:Source; method:"provider_document"|"lexical_clue"|"source_excerpt"; description:string}
    | {kind:"constraint"; control:ControlId; source:Source; title:string; actor_context:string; applicability:{dialect:string;expression:string|null;default_behavior:string}; obligations:Obligation[]; dependencies:ControlId[]; enforcer:string}
    | {kind:"invocation"; parent:Source; callee:Source|null; requested:string; resolution:Resolution; bindings:Binding[]}
    | {kind:"fact"; subject:string; predicate:string; value:Json; authority:string; execution:ExecutionId|null}
    | {kind:"association";subject:string;predicate:string;target:string;context:Record<string,string>;authority:string;resolution:Resolution}
    | {kind:"outcome"; execution:ExecutionId; definition:Source; title:string; status:"queued"|"in_progress"|"completed"|"unknown"; conclusion:string|null; duration_ms:number|null; started_at:string|null; completed_at:string|null; declaration_candidates:ControlId[]; correlation:Resolution};
  interface Model { schema_version:1; scope:string; claim:Claim; entity:Entity }
  /** V1 emitted record envelope; its model can contain declarations or mapped assertions. */
  interface Observation { model:Model; data:Json; receipts:string[] }
  interface GovernanceId { namespace:string; authority:string; local_id:string }
  type GovernanceKind = "policy"|"contract"|"control"|"handoff"|"principal"|"role_assignment"|"contact_method";
  /** Resolved references require an exact revision and a target in the same capture. */
  type GovernanceReference<K extends GovernanceKind = GovernanceKind> = {kind:K;id:GovernanceId} & (
    {resolution:"resolved";source_revision:string} | {resolution:"candidate"|"unknown";source_revision:string|null});
  /** Opaque provider rule text; this SDK does not evaluate fulfillment. */
  interface GovernanceRequirement { id:string; description:string; specification:string }
  interface GovernanceSourceRecord { id:GovernanceId; source:Source }
  type GovernanceRecord = GovernanceSourceRecord & (
    | {type:"policy";title:string;authority:GovernanceReference<"principal">;subjects:string[];requirements:GovernanceRequirement[]}
    | {type:"contract";title:string;parties:[GovernanceReference<"principal">,GovernanceReference<"principal">,...GovernanceReference<"principal">[]];subjects:string[];requirements:GovernanceRequirement[]}
    | {type:"control";title:string;governing_rules:GovernanceReference<"policy"|"contract">[];mechanism:string}
    | {type:"handoff";title:string;policy:GovernanceReference<"policy">;from:GovernanceReference<"principal">;to:GovernanceReference<"principal">;trigger:string;required_action:string;completion:string;continuation:string}
    | {type:"principal";title:string;principal_kind:"human"|"service"|"group"|"role"|"unknown"}
    | {type:"role_assignment";principal:GovernanceReference<"principal">;role:GovernanceReference<"principal">;relationship:"membership"|"responsibility"|"approval_authority"|"delegation";valid_from:number|null;valid_until:number|null}
    | {type:"contact_method";owner:GovernanceReference<"principal">;channel:"email"|"provider_profile"|"team_channel"|"ticket_queue"|"on_call";endpoint:string|null;availability:"available"|"unavailable"|"unknown";purpose:string;valid_until:number|null}
  );
  interface GovernanceModel { schema_version:2; scope:string; claim:"declared"|"provider_reported"; entity:{kind:"governance";record:GovernanceRecord} }
  interface GovernanceObservation { model:GovernanceModel; data:Json; receipts:string[] }
  type EmittedModel = Model|GovernanceModel;
  type EmittedObservation = Observation|GovernanceObservation;
  /** Read-only projection of a stored operation declaration, not an emitted record kind. */
  interface OperationDescription {
    operation:OperationId;
    source:Source;
    record_revision:string;
    title:string;
    claim:Claim;
    scope:string;
    actor_context:string;
    applicability:{dialect:string;expression:string|null;default_behavior:string};
    /** Source label; it does not establish an observed operating control. */
    declared_enforcer:string;
    requirements:NamedRequirement[];
    dependencies:OperationId[];
  }
  interface OperationsResponse {
    schema:"ipe.integration.operations.v2";
    accepted_revision:string|null;
    candidate_revision:string|null;
    operations:OperationDescription[];
  }
  /** Context selects evidence. It is not evidence or an authorization claim. */
  interface AssessmentRequestV2 {
    operation:OperationId;
    source_revision:string;
    record_revision:string;
    context:Record<string,string>;
  }
  interface EvidenceProvenance {
    integration:string;
    key:string;
    revision_id:string;
    run_id:number;
    collector:string;
    observed_at:number;
    expires_at:number|null;
    fresh:boolean;
    receipts:string[];
    claim:Claim|null;
  }
  /** A configured evidence comparison. A match does not establish requirement fulfillment. */
  interface EvidenceCheckV2 {
    status:"matched"|"not_matched"|"unknown";
    method:"configured_evidence_rule";
    reason:string;
    evidence:{provenance:EvidenceProvenance;subject:string;predicate:string;value:Json;authority:string}[];
    associations:{provenance:EvidenceProvenance;target:string;resolution:Resolution;authority:string}[];
    expected?:Json;
    subjects?:string[];
  }
  interface AssessmentResponseV2 {
    schema:"ipe.integration.context-assessment.v2";
    integration:string;
    operation:OperationId;
    source:Source;
    record_revision:string;
    context:Record<string,string>;
    context_authority:"caller_selected_not_evidence";
    as_of:number;
    accepted_revision:string|null;
    requirements:{
      id:string;
      description:string;
      specification:RequirementSpecification;
      rule:string|null;
      rule_digest:string|null;
      evidence_check:EvidenceCheckV2;
      fulfillment:{status:"unknown";reason:string};
    }[];
    status:"unknown";
    applicability:{status:"unknown";reason:string};
    authorization:false;
    publication:false;
    reason:string;
    operation_evidence:EvidenceProvenance;
  }
  interface SDK {
    observation(model:Model, data:Json, receipts:string[]): Observation;
    observation(model:GovernanceModel, data:Json, receipts:string[]): GovernanceObservation;
    observation(model:EmittedModel, data:Json, receipts:string[]): EmittedObservation;
    /** Pure description builder. Native validation checks references and source evidence. */
    governanceModel(record:GovernanceRecord, options:{scope:string;claim:"declared"|"provider_reported"}):GovernanceModel;
    call(id: string, operation: string, params?: Record<string,string>): Request;
    exchange(items: Request[], state?: Json, observations?: EmittedObservation[], progress?: Progress | null): StepResult;
    tasks(state?: TaskState | null, options?: Partial<{queue:number;tasks:number;depth:number;gaps:number}>): TaskPlan;
    observed(receipts: string[], scope: string, claim: Claim): {emit(entity: Entity, data?: Json): Observation};
    readResult(responses: Response[], id: string): ReadResult;
    project(record: Record<string,Json>, fields: string[], excluded?: string[]): Record<string,Json>;
    emitBatch<T, U>(items: T[], map: (item:T) => U, maximum?: number): U[];
    elapsed(start: string, end: string): number | null;
    /** Uses the existing control: key encoding so stored references remain compatible. */
    operationKey(operation: OperationId): string;
    /** V1 compatibility name for operationKey. */
    controlKey(control: ControlId): string;
    executionKey(execution: ExecutionId): string;
    timestamp(value: string): number | null;
    readonly version: "0.7.0";
    readonly limits: { readonly requestsPerStep: 8; readonly resultBytes: 65536; readonly inputBytes: 131072 };
    request(id: string, operation: string, params?: Record<string, string>): Request;
    requests(items: Request[], state?: Json): StepResult;
    response(responses: Response[], id: string): Response;
    records(responses: Response[], id: string): Record<string, Json>[];
    pick(record: Record<string, Json>, fields: string[]): Record<string, Json>;
    complete(outputs: Json, state?: Json): StepResult;
    state(value: Json, initial?: Json): Json;
  }
}
declare const ipe: Ipe.SDK;
