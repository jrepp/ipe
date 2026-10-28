import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { mkdtemp, rm, readdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { gzipSync } from "node:zlib";
import ssri from "ssri";
import { prepare, validateLock, MAX_PACKAGE_BYTES } from "./cache.mjs";

function archive() {
  const data = Buffer.from(JSON.stringify({ name: "fixture", version: "1.0.0",
    scripts: { install: "touch SHOULD_NOT_EXIST" } }));
  const header = Buffer.alloc(512);
  header.write("package/package.json");
  header.write("0000644\0", 100);
  header.write("0000000\0", 108);
  header.write("0000000\0", 116);
  header.write(`${data.length.toString(8).padStart(11, "0")}\0`, 124);
  header.fill(32, 148, 156);
  header[156] = 48;
  header.write("ustar\0", 257);
  header.write(`${header.reduce((n, b) => n + b, 0).toString(8).padStart(6, "0")}\0 `, 148);
  return gzipSync(Buffer.concat([header, data, Buffer.alloc(512 - data.length), Buffer.alloc(1024)]));
}
async function registry(t, bytes = archive()) {
  let hits = 0;
  const server = createServer((request, response) => {
    hits++;
    if (request.url.includes(".tgz")) return response.end(bytes);
    response.setHeader("content-type", "application/json");
    response.end(JSON.stringify({ name: "fixture", versions: { "1.0.0": {
      name: "fixture", version: "1.0.0", dist: {
        tarball: `http://127.0.0.1:${server.address().port}/fixture/-/fixture-1.0.0.tgz`,
        integrity: ssri.fromData(bytes, { algorithms: ["sha512"] }).toString(),
      },
    }}}));
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise(resolve => server.close(resolve)));
  const cache = await mkdtemp(join(tmpdir(), "ipe-packages-"));
  t.after(() => rm(cache, { recursive: true, force: true }));
  const lock = { version: 1, packages: [{ name: "fixture", version: "1.0.0",
    integrity: ssri.fromData(bytes, { algorithms: ["sha512"] }).toString() }] };
  return { cache, lock, registry: `http://127.0.0.1:${server.address().port}/`, hits: () => hits };
}

test("fetches pinned artifacts and replays offline without extracting or running scripts", async t => {
  const fixture = await registry(t);
  const report = await prepare(fixture.lock, fixture);
  assert.equal(report.packages[0].integrity, fixture.lock.packages[0].integrity);
  const hits = fixture.hits();
  assert.ok(hits >= 2);
  assert.deepEqual(await prepare(fixture.lock, { ...fixture, offline: true }), report);
  assert.equal(fixture.hits(), hits);
  const files = await readdir(fixture.cache, { recursive: true });
  assert.equal(files.some(file => file.endsWith("package.json") || file.endsWith("SHOULD_NOT_EXIST")), false);
  const content = files.find(file => file.startsWith("artifacts/content-v2/") && file.split("/").length === 6);
  assert.ok(content);
  await writeFile(join(fixture.cache, content), "corrupt");
  await assert.rejects(prepare(fixture.lock, { ...fixture, offline: true }), /offline_miss_or_corrupt/);
  assert.equal(fixture.hits(), hits);
});
test("rejects wrong pins, uncached offline artifacts and oversized packages", async t => {
  const fixture = await registry(t);
  await assert.rejects(prepare(fixture.lock, { ...fixture, offline: true }), /offline_miss_or_corrupt/);
  assert.equal(fixture.hits(), 0);
  fixture.lock.packages[0].integrity = ssri.fromData("different", { algorithms: ["sha512"] }).toString();
  await assert.rejects(prepare(fixture.lock, fixture));
  const large = await registry(t, Buffer.alloc(MAX_PACKAGE_BYTES + 1, 1));
  await assert.rejects(prepare(large.lock, large), /package_size_limit/);
});
test("lock accepts only explicit registry versions with SHA-512 pins", () => {
  const integrity = ssri.fromData("fixture", { algorithms: ["sha512"] }).toString();
  for (const version of ["latest", "^1.0.0", "git+https://example.com/repo", "file:../x"]) {
    assert.throws(() => validateLock({ version: 1, packages: [{ name: "fixture", version, integrity }] }));
  }
  for (const name of ["../fixture", "https://example.com/x", "fixture@1.0.0"]) {
    assert.throws(() => validateLock({ version: 1, packages: [{ name, version: "1.0.0", integrity }] }));
  }
});
