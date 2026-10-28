import { open } from "node:fs/promises";
import { parseArgs } from "node:util";
import { prepare } from "./cache.mjs";

try {
  const { values } = parseArgs({ options: {
    lock: { type: "string" }, cache: { type: "string" }, registry: { type: "string" },
    offline: { type: "boolean", default: false },
  }});
  if (!values.lock || !values.cache) throw new Error("arguments");
  const file = await open(values.lock, "r");
  let bytes;
  try {
    const buffer = Buffer.alloc(65537);
    let length = 0;
    while (length < buffer.length) {
      const read = await file.read(buffer, length, buffer.length - length, null);
      if (!read.bytesRead) break;
      length += read.bytesRead;
    }
    if (length > 65536) throw new Error("lock_limit");
    bytes = buffer.subarray(0, length);
  } finally { await file.close(); }
  const report = await prepare(JSON.parse(bytes.toString("utf8")), { ...values, token: process.env.IPE_NPM_TOKEN });
  process.stdout.write(`${JSON.stringify(report)}\n`);
} catch {
  // Registry URLs, authentication details and untrusted package metadata stay out of logs.
  process.stderr.write("integration_package_cache_failed\n");
  process.exitCode = 1;
}
