// Trusted build-side cache. This module is never loaded into QuickJS.
import pacote from "pacote";
import cacache from "cacache";
import ssri from "ssri";
import { join } from "node:path";

export const MAX_PACKAGE_BYTES = 8 * 1024 * 1024;
const invalid = () => { throw new Error("package_lock_invalid"); };
export function validateLock(lock) {
  if (!lock || lock.version !== 1 || !Array.isArray(lock.packages) || lock.packages.length > 64 ||
      Object.keys(lock).some(key => !["version", "packages"].includes(key))) invalid();
  const seen = new Set();
  for (const item of lock.packages) {
    if (!item || Object.keys(item).sort().join(",") !== "integrity,name,version" ||
        typeof item.name !== "string" || item.name.length > 214 ||
        !/^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/.test(item.name) ||
        typeof item.version !== "string" || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(item.version) ||
        typeof item.integrity !== "string" || !/^sha512-[A-Za-z0-9+/]{86}==$/.test(item.integrity)) invalid();
    const key = `${item.name}@${item.version}`;
    if (seen.has(key)) invalid();
    seen.add(key);
  }
  return lock;
}

async function bounded(stream) {
  const chunks = [];
  let length = 0;
  for await (const chunk of stream) {
    length += chunk.length;
    if (length > MAX_PACKAGE_BYTES) throw new Error("package_size_limit");
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

export async function prepare(lock, { cache, registry = "https://registry.npmjs.org/", offline = false, token } = {}) {
  validateLock(lock);
  if (typeof cache !== "string" || !cache) throw new Error("package_cache_required");
  const url = new URL(registry);
  if (url.username || url.password || url.search || url.hash ||
      (url.protocol !== "https:" && !(url.protocol === "http:" && ["127.0.0.1", "[::1]"].includes(url.hostname)))) {
    throw new Error("package_registry_invalid");
  }
  if (token && (url.protocol !== "https:" || typeof token !== "string" || /[\r\n]/.test(token))) {
    throw new Error("package_registry_credentials_invalid");
  }
  if (!url.pathname.endsWith("/")) url.pathname += "/";
  const authentication = token ? { [`//${url.host}${url.pathname}:_authToken`]: token } : {};
  const artifacts = join(cache, "artifacts");
  const result = [];
  for (const item of lock.packages) {
    let bytes;
    try {
      bytes = await bounded(cacache.get.stream.byDigest(artifacts, item.integrity));
    } catch (error) {
      if (offline) throw new Error("package_offline_miss_or_corrupt");
      if (!["ENOENT", "EINTEGRITY"].includes(error.code)) throw error;
      bytes = await pacote.tarball.stream(`${item.name}@${item.version}`, bounded, {
        ...authentication,
        // Only our verified artifact cache writes bytes. Avoid a second cache
        // writer continuing after the bounded consumer rejects a stream.
        cache: false, preferOnline: true, registry: url.href, integrity: item.integrity,
        ignoreScripts: true, offline: false, fetchRetries: 0, fetchTimeout: 15000,
      });
    }
    if (!ssri.checkData(bytes, item.integrity)) throw new Error("package_integrity_mismatch");
    await cacache.put(artifacts, `${item.name}@${item.version}`, bytes, { integrity: item.integrity });
    result.push({ ...item, bytes: bytes.length });
  }
  return { version: 1, packages: result };
}
