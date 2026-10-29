// Host-supplied, pure helpers for json-step-v1. No I/O is performed here.
(() => {
  "use strict";
  const limits = Object.freeze({ requestsPerStep: 8, resultBytes: 65536, inputBytes: 131072 });
  const fail = code => { throw new Error(code); };
  const token = (value, max) => {
    if (typeof value !== "string" || value.length < 1 || value.length > max) return false;
    for (let index = 0; index < value.length; index++) {
      const code = value.charCodeAt(index);
      if (!(code >= 48 && code <= 57) && !(code >= 65 && code <= 90)
        && !(code >= 97 && code <= 122) && code !== 95) return false;
    }
    return true;
  };
  const name = value => token(value, 64);
  function json(value) {
    const seen = new Set();
    const pending = [{ item: value, depth: 0, exit: false }];
    while (pending.length) {
      const frame = pending.pop();
      const item = frame.item;
      if (frame.exit) {
        seen.delete(item);
        continue;
      }
      if (frame.depth > 32) fail("ipe_json_depth");
      if (item === null || typeof item === "string" || typeof item === "boolean") continue;
      if (typeof item === "number" && Number.isFinite(item)) continue;
      if (typeof item !== "object" || seen.has(item)) fail("ipe_json_invalid");
      if (!Array.isArray(item) && Object.getPrototypeOf(item) !== Object.prototype && Object.getPrototypeOf(item) !== null) fail("ipe_json_invalid");
      seen.add(item);
      pending.push({ item, depth: frame.depth, exit: true });
      const keys = Object.keys(item);
      for (let index = keys.length - 1; index >= 0; index--) {
        pending.push({ item: item[keys[index]], depth: frame.depth + 1, exit: false });
      }
    }
    const text = JSON.stringify(value);
    // JSON escapes lone surrogates; count UTF-8 without needing host TextEncoder.
    let bytes = 0;
    for (const char of text) {
      const point = char.codePointAt(0);
      bytes += point < 128 ? 1 : point < 2048 ? 2 : point < 65536 ? 3 : 4;
    }
    if (bytes > limits.resultBytes) fail("ipe_json_limit");
    return JSON.parse(text);
  }
  function request(id, operation, params = {}) {
    if (!name(id) || !name(operation)) fail("ipe_request_invalid");
    const copy = json(params);
    if (!copy || Array.isArray(copy) || typeof copy !== "object" || Object.keys(copy).length > 8) fail("ipe_parameters_invalid");
    for (const [key, value] of Object.entries(copy)) {
      if (!name(key) || !token(value, 128)) fail("ipe_parameters_invalid");
    }
    return { id, operation, params: copy };
  }
  function requests(items, state = null) {
    if (!Array.isArray(items) || items.length < 1 || items.length > limits.requestsPerStep) fail("ipe_requests_invalid");
    const ids = new Set();
    const validated = items.map(item => {
      const value = request(item.id, item.operation, item.params);
      if (ids.has(value.id)) fail("ipe_request_duplicate");
      ids.add(value.id);
      return value;
    });
    return json({ requests: validated, state });
  }
  function response(responses, id) {
    if (!Array.isArray(responses)) fail("ipe_responses_invalid");
    const matches = responses.filter(item => item.id === id);
    if (matches.length !== 1) fail("ipe_response_missing_or_duplicate");
    return matches[0];
  }
  function records(responses, id) {
    const item = response(responses, id);
    if (item.code !== undefined) fail("ipe_response_failed");
    if (!Array.isArray(item.data) || typeof item.receipt !== "string") fail("ipe_response_invalid");
    return json(item.data);
  }
  function pick(record, fields) {
    if (!record || typeof record !== "object" || Array.isArray(record) || !Array.isArray(fields)) fail("ipe_projection_invalid");
    const result = Object.create(null);
    for (const field of fields) {
      if (typeof field !== "string") fail("ipe_projection_invalid");
      if (Object.hasOwn(record, field)) result[field] = record[field];
    }
    return json(result);
  }
  // Broader parameter strings are still subject to native operation grants.
  function call(id, operation, params = {}) {
    if (!name(id) || !name(operation)) fail("ipe_request_invalid");
    const copy = json(params);
    if (!copy || Array.isArray(copy) || typeof copy !== "object" || Object.keys(copy).length > 8) fail("ipe_parameters_invalid");
    for (const [key, value] of Object.entries(copy)) {
      if (!name(key) || typeof value !== "string" || value.length > 2048 || value.includes('\0')) fail("ipe_parameters_invalid");
    }
    return { id, operation, params: copy };
  }
  function exchange(items, state = null, observations = []) {
    if (!Array.isArray(items) || items.length > 8 || !Array.isArray(observations) || observations.length > 16) fail("ipe_requests_invalid");
    const ids = new Set();
    const validated = items.map(item => {
      const request = call(item.id, item.operation, item.params);
      if (ids.has(request.id)) fail("ipe_request_duplicate");
      ids.add(request.id); return request;
    });
    return json({requests: validated, state, observations});
  }
  // Capture only deterministic parsing. The host removes Date before guest code.
  const parseTimestamp = Date.parse.bind(Date);
  function timestamp(value) {
    if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value)) return null;
    const result = parseTimestamp(value);
    return Number.isFinite(result) ? result : null;
  }
  function observation(model, data, receipts) {
    if (!model || model.schema_version !== 1 || !Array.isArray(receipts) || receipts.length<1 || receipts.length>8 || receipts.some(r=>typeof r!=='string')) fail("ipe_observation_invalid");
    return json({model,data,receipts});
  }
  const sdk = Object.freeze({ version: "0.3.0", observation, call, exchange, timestamp, limits, request, requests, response, records, pick,
    complete: (outputs, state = null) => json({ outputs, state }),
    state: (value, initial = null) => json(value === null ? initial : value),
  });
  Object.defineProperty(globalThis, "ipe", { value: sdk, writable: false, configurable: false });
})();
