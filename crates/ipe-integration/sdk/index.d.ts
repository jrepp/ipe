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
  interface Observation { kind: "contract" | "fact" | "outcome"; key: string; data: Json; receipts: string[] }
  interface SDK {
    call(id: string, operation: string, params?: Record<string,string>): Request;
    exchange(items: Request[], state?: Json, observations?: Observation[]): StepResult;
    timestamp(value: string): number | null;
    readonly version: "0.2.0";
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
