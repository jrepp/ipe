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
  function exchange(items, state = null, observations = [], progress = null) {
    if (!Array.isArray(items) || items.length > 8 || !Array.isArray(observations) || observations.length > 16) fail("ipe_requests_invalid");
    const ids = new Set();
    const validated = items.map(item => {
      const request = call(item.id, item.operation, item.params);
      if (ids.has(request.id)) fail("ipe_request_duplicate");
      ids.add(request.id); return request;
    });
    return json({requests: validated, state, observations, ...(progress===null?{}:{progress})});
  }
  // Capture only deterministic parsing. The host removes Date before guest code.
  const parseTimestamp = Date.parse.bind(Date);
  function timestamp(value) {
    if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value)) return null;
    const result = parseTimestamp(value);
    return Number.isFinite(result) ? result : null;
  }
  // Build descriptions only. Native validation checks complete schemas and same-capture references.
  function governanceModel(record, options) {
    const text = value => typeof value === 'string' && value.length > 0 && value.length <= 2048 && !/[\u0000-\u001f\u007f]/.test(value);
    const source = record && record.source, id = record && record.id;
    if (!record || !['policy','contract','control','handoff','principal','role_assignment','contact_method'].includes(record.type)
      || !id || ![id.namespace,id.authority,id.local_id].every(text)
      || !source || !source.definition || ![source.definition.namespace,source.definition.resource,source.definition.path,source.revision].every(text)
      || typeof source.pointer !== 'string' || !options || !text(options.scope)
      || !['declared','provider_reported'].includes(options.claim)) fail('ipe_governance_invalid');
    return json({schema_version:2,scope:options.scope,claim:options.claim,entity:{kind:'governance',record}});
  }
  function observation(model, data, receipts) {
    if (!model || (model.schema_version !== 1 && !(model.schema_version === 2 && model.entity && model.entity.kind === "governance")) || !Array.isArray(receipts) || receipts.length<1 || receipts.length>8 || receipts.some(r=>typeof r!=='string')) fail("ipe_observation_invalid");
    return json({model,data,receipts});
  }
  // Pure per-run work plan. Only JSON state crosses sandbox steps.
  function tasks(value = null, options = {}) {
    const bounds = { queue: 64, tasks: 80, depth: 4, gaps: 40, ...options };
    for (const [key, maximum] of Object.entries({queue:64,tasks:80,depth:4,gaps:40})) {
      if (!Number.isInteger(bounds[key]) || bounds[key] < 1 || bounds[key] > maximum) fail('ipe_task_limit_invalid');
    }
    const state = value === null ? {queue:[],seen:[],sequence:0,current:null,gaps:[],droppedGaps:0} : value;
    if (!state || !Array.isArray(state.queue) || !Array.isArray(state.seen) || !Array.isArray(state.gaps)
      || state.queue.length > bounds.queue || state.seen.length > bounds.tasks || state.gaps.length > bounds.gaps
      || !Number.isSafeInteger(state.sequence) || state.sequence < 0 || !Number.isSafeInteger(state.droppedGaps) || state.droppedGaps < 0) fail('ipe_task_state_invalid');
    const gap = (task, code) => {
      if (!name(code)) fail('ipe_gap_invalid');
      if (state.gaps.length < bounds.gaps) state.gaps.push({task,code});
      else state.droppedGaps++;
    };
    return Object.freeze({state, gap,
      enqueue(task, key, depth = 0) {
        if (typeof key !== 'string' || !key.length || key.length > 4096 || !Number.isInteger(depth) || depth < 0) fail('ipe_task_invalid');
        if (state.seen.includes(key) || state.queue.some(item=>item.key===key)) return false;
        if (depth > bounds.depth || state.queue.length >= bounds.queue) { gap(task,'traversal_limit'); return false; }
        state.queue.push({task,key}); return true;
      },
      next() {
        if (state.current) fail('ipe_task_response_pending');
        if (!state.queue.length) return null;
        if (state.seen.length >= bounds.tasks) { gap(null,'request_plan_limit'); state.queue=[]; return null; }
        const item=state.queue.shift();state.seen.push(item.key);return item.task;
      },
      request(task, operation, params) {
        if (state.current) fail('ipe_task_response_pending');
        const id='task_'+(++state.sequence);
        const result=call(id,operation,params);state.current={task,id};return result;
      },
      receive(responses) {
        if (!state.current) return null;
        const current=state.current,result=response(responses,current.id);
        state.current=null;return {task:current.task,result};
      },
      progress(phase, task = null) {
        if (!['dispatch','response','emit','complete'].includes(phase) || (task !== null && !name(task))) fail('ipe_progress_invalid');
        return {phase,task,queued:state.queue.length,completed:state.seen.length-(state.current?1:0),gaps:state.gaps.length+state.droppedGaps};
      },
    });
  }
  function observed(receipts, scope, claim) {
    if (!Array.isArray(receipts) || !receipts.length || receipts.length > 8 || receipts.some(r=>typeof r!=='string'||!r.length)
      || typeof scope !== 'string' || !scope.length || !['declared','provider_reported','inferred','unknown'].includes(claim)) fail('ipe_observation_invalid');
    const bound=receipts.slice();
    return Object.freeze({emit:(entity,data=null)=>observation({schema_version:1,scope,claim,entity},data,bound)});
  }
  function readResult(responses, id) {
    const result=response(responses,id);
    if (result.code !== undefined) return result;
    const rows=records(responses,id);
    return Object.freeze({...result,data:rows,
      observations:(scope,claim)=>observed([result.receipt],scope,claim),
    });
  }
  function project(record, fields, excluded = []) {
    if (!record || typeof record !== 'object' || Array.isArray(record) || !Array.isArray(fields) || !Array.isArray(excluded)
      || [...fields,...excluded].some(field=>typeof field!=='string')) fail('ipe_projection_invalid');
    // Keep references within this step. exchange/complete validate the final wire value.
    const result=Object.create(null),rest=Object.create(null);
    for (const key of Object.keys(record)) {
      if (fields.includes(key)) result[key]=record[key];
      else if (!excluded.includes(key)) rest[key]=record[key];
    }
    if (Object.keys(rest).length) result.unmodeled=rest;
    return result;
  }
  function emitBatch(items, map, maximum = 1) {
    if (!Array.isArray(items) || typeof map !== 'function' || !Number.isInteger(maximum) || maximum<1 || maximum>16) fail('ipe_batch_invalid');
    const output=items.slice(0,maximum).map(map);items.splice(0,output.length);return output;
  }
  function elapsed(start, end) {
    const a=timestamp(start),b=timestamp(end);return a===null||b===null||b<a?null:b-a;
  }
  // Keep the v1 encoding: existing associations use these operation identities.
  function operationKey(operation) { return "control:"+JSON.stringify([operation.namespace,operation.resource,operation.operation,operation.local_id]); }
  function controlKey(control) { return operationKey(control); }
  function executionKey(e) {
    return 'execution:'+JSON.stringify([e.namespace,e.resource,e.run,e.attempt,e.job,e.step]);
  }
  const sdk = Object.freeze({ version: "0.7.0", governanceModel, operationKey, controlKey, tasks, observed, readResult, project, emitBatch, elapsed, executionKey, observation, call, exchange, timestamp, limits, request, requests, response, records, pick,
    complete: (outputs, state = null) => json({ outputs, state }),
    state: (value, initial = null) => json(value === null ? initial : value),
  });
  Object.defineProperty(globalThis, "ipe", { value: sdk, writable: false, configurable: false });
})();
