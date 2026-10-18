/** Independent, bounded JSON-RPC wire transport for synthetic deployment validation. */
import { spawn } from 'node:child_process';
import type { ChildProcessWithoutNullStreams } from 'node:child_process';

export const RESPONSE_LIMIT = 8 * 1024 * 1024;
export async function boundedResponse(response: Response, limit = RESPONSE_LIMIT): Promise<string> {
  if (!response.body) return '';
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > limit) throw Error('response_limit');
      chunks.push(value);
    }
    return Buffer.concat(chunks).toString('utf8');
  } finally {
    await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}

type Message = Record<string, any>;
type Pending = { resolve: (value: Message) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> };
export class WireClient {
  readonly child: ChildProcessWithoutNullStreams;
  private pending = new Map<number, Pending>();
  private nextId = 1;
  private buffer = Buffer.alloc(0);
  private outputBytes = 0;
  private stderrBytes = 0;
  private failure?: Error;
  private ended = false;
  private exitCode: number | null = null;
  private closed: Promise<void>;
  private timeout: number;
  private frameLimit: number;
  private totalLimit: number;
  private stderrLimit: number;
  private inspect: (value: unknown) => void;

  constructor(binary: string, args: string[], env: NodeJS.ProcessEnv, inspect: (value: unknown) => void,
    limits: { timeoutMs?: number; frameBytes?: number; totalBytes?: number; stderrBytes?: number } = {}) {
    this.timeout = limits.timeoutMs ?? 15000;
    this.inspect = inspect;
    this.frameLimit = limits.frameBytes ?? RESPONSE_LIMIT;
    this.totalLimit = limits.totalBytes ?? 32 * 1024 * 1024;
    this.stderrLimit = limits.stderrBytes ?? 64 * 1024;
    this.child = spawn(binary, args, { env, stdio: ['pipe', 'pipe', 'pipe'] });
    this.closed = new Promise(resolve => {
      this.child.once('close', code => {
        this.ended = true; this.exitCode = code;
        if (this.pending.size || this.buffer.length) this.fail('incomplete_response');
        resolve();
      });
    });
    this.child.on('error', () => this.fail('child_failed'));
    this.child.stdin.on('error', () => this.fail('stdin_failed'));
    this.child.stdout.on('error', () => this.fail('stdout_failed'));
    this.child.stderr.on('error', () => this.fail('stderr_failed'));
    let stderr = '';
    this.child.stderr.on('data', (chunk: Buffer) => {
      this.stderrBytes += chunk.length;
      if (this.stderrBytes > this.stderrLimit) return this.fail('stderr_limit');
      stderr += chunk.toString('utf8');
      try { this.inspect(stderr); } catch { this.fail('stderr_disclosure'); }
    });
    this.child.stdout.on('data', (chunk: Buffer) => {
      if (this.failure) return;
      this.outputBytes += chunk.length;
      if (this.outputBytes > this.totalLimit) return this.fail('stdout_limit');
      this.buffer = Buffer.concat([this.buffer, chunk]);
      for (;;) {
        const end = this.buffer.indexOf(10);
        if (end < 0) {
          if (this.buffer.length > this.frameLimit) this.fail('frame_limit');
          return;
        }
        if (end > this.frameLimit) return this.fail('frame_limit');
        const line = this.buffer.subarray(0, end).toString('utf8');
        this.buffer = this.buffer.subarray(end + 1);
        try {
          this.inspect(line);
          const message: Message = JSON.parse(line);
          if (!message || message.jsonrpc !== '2.0' || typeof message.id !== 'number') throw Error();
          const item = this.pending.get(message.id);
          if (!item || (('result' in message) === ('error' in message))) throw Error();
          clearTimeout(item.timer); this.pending.delete(message.id); item.resolve(message);
        } catch { return this.fail('invalid_response'); }
      }
    });
  }
  private fail(code: string) {
    this.failure ??= Error(code);
    for (const item of this.pending.values()) { clearTimeout(item.timer); item.reject(this.failure); }
    this.pending.clear();
    this.buffer = Buffer.alloc(0);
    if (!this.ended) this.child.kill('SIGKILL');
  }
  send(method: string, params: unknown): Promise<Message> {
    if (this.failure || this.ended) return Promise.reject(this.failure ?? Error('child_closed'));
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => this.fail('response_timeout'), this.timeout);
      this.pending.set(id, { resolve, reject, timer });
      this.child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
    });
  }
  notify(method: string) {
    if (this.failure || this.ended) throw this.failure ?? Error('child_closed');
    this.child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method }) + '\n');
  }
  private async waitBounded(timeout: number) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      await Promise.race([this.closed, new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(Error('child_close_timeout')), timeout);
      })]);
    } finally { clearTimeout(timer); }
  }
  async finish() {
    this.child.stdin.end();
    try { await this.waitBounded(this.timeout); }
    catch { this.fail('exit_timeout'); await this.waitBounded(2000); }
    if (this.failure) throw this.failure;
    if (this.exitCode !== 0) throw Error('child_exit_failed');
  }
  async close() {
    if (!this.ended) this.fail('client_closed');
    await this.waitBounded(2000);
  }
}
