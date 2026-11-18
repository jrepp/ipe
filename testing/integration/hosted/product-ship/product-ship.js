(function (root, factory) {
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.IpeProductShip = api;
})(typeof globalThis === 'object' ? globalThis : this, function () {
  'use strict';

  function copy(value) { return JSON.parse(JSON.stringify(value)); }
  function parseTime(value) {
    if (typeof value !== 'string') return null;
    const time = Date.parse(value);
    return Number.isFinite(time) ? time : null;
  }

  function scenarioEvidence(evidence, scenario, release, asOf) {
    const items = copy(evidence || []);
    const firstId = items[0] && items[0].control_id;
    if (scenario === 'ready' || scenario == null) return items;
    if (scenario === 'blocked') {
      const item = items.find(row => row.control_id === firstId);
      if (item) { item.status = 'not_met'; item.reason = 'Synthetic scenario: required check is not met.'; }
    } else if (scenario === 'missing') {
      return items.filter(row => row.control_id !== firstId);
    } else if (scenario === 'expired') {
      const item = items.find(row => row.control_id === firstId);
      if (item) item.expires_at = new Date(parseTime(asOf) - 1000).toISOString();
    } else if (scenario === 'wrong-artifact') {
      const item = items.find(row => row.control_id === firstId);
      if (item) item.artifact = 'sha256:' + '0'.repeat(64);
    }
    return items;
  }

  function simulateScenario(model, scenario) {
    if (!model || !model.release || !model.policy || !Array.isArray(model.policies) || !Array.isArray(model.evidence)) {
      throw new TypeError('invalid_product_ship_model');
    }
    const result = copy(model);
    result.evidence = scenarioEvidence(result.evidence, scenario, result.release, result.as_of);
    return result;
  }

  function evaluate(model, scenario) {
    if (!model || !model.release || !model.policy || !Array.isArray(model.policies) || !Array.isArray(model.evidence)) {
      throw new TypeError('invalid_product_ship_model');
    }
    const release = model.release;
    const asOf = parseTime(model.as_of);
    if (asOf === null) throw new TypeError('invalid_product_ship_as_of');
    const evidence = simulateScenario(model, scenario).evidence;
    const checks = [];

    const policyCounts = new Map();
    if (!Array.isArray(model.policy.children) || model.policy.children.length === 0) {
      checks.push({policy_id:model.policy.id,control_id:'policy-tree',status:'unknown',reason:'The central policy has no required child policies.'});
    }
    const childIds = new Set();
    for (const policy of model.policies) policyCounts.set(policy.id, (policyCounts.get(policy.id) || 0) + 1);
    const seenControlIds = new Set();
    for (const policyId of model.policy.children || []) {
      if (childIds.has(policyId)) {
        checks.push({policy_id:policyId,control_id:'policy:'+policyId,status:'unknown',reason:'The central policy lists a child policy more than once.'});
        continue;
      }
      childIds.add(policyId);
      const matchesForPolicy = model.policies.filter(item => item.id === policyId && item.parent_id === model.policy.id);
      if (matchesForPolicy.length !== 1 || policyCounts.get(policyId) !== 1) {
        checks.push({policy_id:policyId,control_id:'policy:'+policyId,status:'unknown',reason:'Required child policy is missing, duplicated, or has an invalid parent link.'});
        continue;
      }
      const policy = matchesForPolicy[0];
      if (!Array.isArray(policy.controls) || policy.controls.length === 0) {
        checks.push({policy_id:policy.id,control_id:'controls:'+policy.id,status:'unknown',reason:'A required child policy has no required controls.'});
      }
      for (const control of policy.controls || []) {
        if (seenControlIds.has(control.id)) {
          checks.push({policy_id:policy.id,control_id:control.id,status:'unknown',reason:'Duplicate control identity exists in the required policy tree.'});
          continue;
        }
        seenControlIds.add(control.id);
        const matches = evidence.filter(item => item.control_id === control.id && item.release_id === release.id);
        let status = 'unknown';
        let reason = 'No evidence exists for this release and control.';
        let matchedEvidence;
        if (matches.length > 1) {
          reason = 'Conflicting or duplicate evidence exists for this release and control.';
        } else if (matches.length === 1) {
          const item = matches[0];
          matchedEvidence = item;
          if (item.artifact !== release.artifact) reason = 'Evidence artifact does not match the release artifact.';
          else if (item.environment !== release.environment) reason = 'Evidence environment does not match the release environment.';
          else if (item.source !== control.source) reason = 'Evidence source does not match the control source.';
          else if (parseTime(item.observed_at) === null || parseTime(item.observed_at) > asOf) reason = 'Evidence observation time is invalid or later than the model time.';
          else if (parseTime(item.expires_at) === null || parseTime(item.expires_at) <= asOf) reason = 'Evidence has expired or has an invalid expiry time.';
          else if (!['met', 'not_met', 'unknown'].includes(item.status)) reason = 'Evidence has an unsupported status.';
          else { status = item.status; reason = item.reason || (status === 'met' ? 'Required evidence is met.' : status === 'not_met' ? 'Required evidence is not met.' : 'Evidence status is unknown.'); }
        }
        checks.push({policy_id: policy.id, control_id: control.id, status, reason, ...(matchedEvidence ? {evidence: matchedEvidence} : {})});
      }
    }
    const notMet = checks.filter(check => check.status === 'not_met');
    const unknown = checks.filter(check => check.status === 'unknown');
    const decision = notMet.length ? 'deny' : unknown.length ? 'indeterminate' : 'allow';
    const reasons = notMet.length
      ? notMet.map(check => ({policy_id: check.policy_id, control_id: check.control_id, code: 'required_control_not_met', message: check.reason}))
      : unknown.map(check => ({policy_id: check.policy_id, control_id: check.control_id, code: 'required_evidence_unknown', message: check.reason}));
    return {decision, reasons, checks};
  }

  return Object.freeze({evaluate,simulateScenario});
});
