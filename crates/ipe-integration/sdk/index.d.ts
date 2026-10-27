/** Pure helpers provided by the host; transport and credentials remain native. */
declare namespace Ipe {
  type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
  interface Context {
    connection_ref: string;
    operations: { name: string; parameters: string[] }[];
    sdk_version: string;
  }
  interface Request { id: string; operation: string; params: Record<string, string> }
  type Response =
    | { id: string; operation: string; receipt: string; data: Record<string, Json>[]; code?: never }
    | { id: string; operation: string; code: string; data?: never; receipt?: never };
  type StepResult = { requests: Request[]; state: Json } | { outputs: Json; state: Json };
  interface SDK {
    readonly version: "0.1.0";
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
