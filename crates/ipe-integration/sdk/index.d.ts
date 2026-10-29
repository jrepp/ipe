/** Pure helpers provided by the host; transport and credentials remain native. */
declare namespace Ipe {
  type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
  interface Context {
    connection_ref: string;
    operations: { name: string; parameters: string[] }[];
    sdk_version: string;
  }
  interface HostedContext {
    integration_id: string;
    run_id: number;
    observed_at: number;
    input: Json;
    operations: string[];
    sdk_version: string;
  }
  interface Request { id: string; operation: string; params: Record<string, string> }
  type Response =
    | { id: string; operation: string; receipt: string; data: Record<string, Json>[]; source_digest?: string; truncated?: boolean; code?: never }
    | { id: string; operation: string; code: string; data?: never; receipt?: never };
  type StepResult = { requests: Request[]; state: Json; observations?: Observation[] } | { outputs: Json; state: Json };
  interface DefinitionId { namespace: string; resource: string; path: string }
  interface Source { definition: DefinitionId; revision: string; pointer: string }
  interface ControlId { namespace: string; resource: string; operation: string; local_id: string }
  interface ExecutionId { namespace: string; resource: string; run: string; attempt: number; job: string | null; step: number | null }
  type Claim = "declared" | "provider_reported" | "inferred" | "unknown";
  type Resolution = "resolved" | "candidate" | "unknown";
  interface Binding { name: string; origin: "caller" | "declared_default" | "absent" | "expression"; value: Json }
  type Requirement =
    | {kind:"fact"; query:{subject:string; predicate:string; expected:Json}}
    | {kind:"opaque"; reason:string}
    | {kind:"permission"; requested:Json}
    | {kind:"environment"; reference:Json}
    | {kind:"dependency"; control:ControlId};
  interface Obligation { id:string; description:string; requirement:Requirement; next_observation:{operation:string;reason:string}|null }
  type Entity =
    | {kind:"definition"; source:Source; title:string; inputs:{name:string;required:boolean|null;default:Json}[]; outputs:string[]}
    | {kind:"evidence"; source:Source; method:"provider_document"|"lexical_clue"|"source_excerpt"; description:string}
    | {kind:"constraint"; control:ControlId; source:Source; title:string; actor_context:string; applicability:{dialect:string;expression:string|null;default_behavior:string}; obligations:Obligation[]; dependencies:ControlId[]; enforcer:string}
    | {kind:"invocation"; parent:Source; callee:Source|null; requested:string; resolution:Resolution; bindings:Binding[]}
    | {kind:"fact"; subject:string; predicate:string; value:Json; authority:string; execution:ExecutionId|null}
    | {kind:"outcome"; execution:ExecutionId; definition:Source; title:string; status:"queued"|"in_progress"|"completed"|"unknown"; conclusion:string|null; duration_ms:number|null; started_at:string|null; completed_at:string|null; declaration_candidates:ControlId[]; correlation:Resolution};
  interface Model { schema_version:1; scope:string; claim:Claim; entity:Entity }
  interface Observation { model:Model; data:Json; receipts:string[] }
  interface SDK {
    observation(model:Model, data:Json, receipts:string[]): Observation;
    call(id: string, operation: string, params?: Record<string,string>): Request;
    exchange(items: Request[], state?: Json, observations?: Observation[]): StepResult;
    timestamp(value: string): number | null;
    readonly version: "0.3.0";
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
