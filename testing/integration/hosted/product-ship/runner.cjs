#!/usr/bin/env node
// Deterministic broker replay for product ship qualification domain collectors.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const here = __dirname;
const fixture = JSON.parse(fs.readFileSync(path.join(here, 'product-ship.json'), 'utf8'));
const sdk = fs.readFileSync(path.resolve(here, '../../../../crates/ipe-integration/sdk/index.js'), 'utf8');
const collectorPath = path.join(here, 'collector.js');
const collector = fs.readFileSync(collectorPath, 'utf8');
const provenanceRunner = require('./provenance-runner.cjs');
require('./product-ship.js');
const productShip = globalThis.IpeProductShip;
const scenarioNames = ['ready', 'blocked', 'missing', 'expired', 'wrong-artifact'];

function providerSnapshot(scenario, policyId = null) {
  const release = {...fixture.release};
  let evidence = fixture.evidence.map(item => ({...item}));
  const first = evidence[0];
  if (scenario === 'blocked') { first.status = 'not_met'; first.reason = 'Synthetic provider reports this required control is not met.'; }
  if (scenario === 'missing') evidence = evidence.filter(item => item.control_id !== first.control_id);
  if (scenario === 'expired') first.expires_at = new Date(Date.parse(fixture.as_of) - 1000).toISOString();
  if (scenario === 'wrong-artifact') first.artifact = 'sha256:' + '0'.repeat(64);
  const policies = policyId === null ? fixture.policies : fixture.policies.filter(policy => policy.id === policyId);
  if (policyId !== null) {
    const selected = new Set((policies[0]?.controls || []).map(control => control.id));
    evidence = evidence.filter(item => selected.has(item.control_id));
  }
  return {release,policy:fixture.policy,policies,evidence};
}

function replay(scenario = 'ready', policyId = null, {includeExtraData = false} = {}) {
  let state = null;
  let responses = [];
  const observations = [];
  const requests = [];
  const input = {release:fixture.release,...(policyId === null ? {} : {policy_id:policyId})};
  for (let step = 0; step < 32; step++) {
    const args = JSON.stringify([{input,operations:['read_release_qualification']},responses,state]);
    const wrapped = `${sdk}\n${collector}\nJSON.stringify(step(...JSON.parse(__ipe_args)))`;
    const result = JSON.parse(vm.runInNewContext(wrapped, {__ipe_args:args}, {timeout:1000}));
    observations.push(...(result.observations || []));
    if ('outputs' in result) return {observations,requests,summary:result.outputs};
    state = result.state;
    responses = (result.requests || []).map(request => {
      requests.push(request);
      if (request.operation !== 'read_release_qualification') throw new Error('unexpected_operation');
      if (scenario === 'denied') return {id:request.id,operation:request.operation,code:'provider_denied'};
      const data = providerSnapshot(scenario,includeExtraData ? null : policyId);
      return {id:request.id,operation:request.operation,data:[data],receipt:'fixture:qualification:'+data.release.id,source_digest:'synthetic-qualification-v1'};
    });
  }
  throw new Error('product_ship_step_limit');
}

function modelFromCaptures(captures) {
  const model = JSON.parse(JSON.stringify(fixture));
  model.evidence = captures.flatMap(capture => capture.observations
    .filter(observation => observation.model && observation.model.schema_version === 1 && observation.model.entity.kind === 'fact')
    .map(observation => ({...observation.data,receipt:observation.receipts[0]})));
  return model;
}

function run() {
  const integrations = fixture.policies.map(policy => {
    const config = {
      id:'product-ship-'+policy.id,
      program: path.relative(path.resolve(here,'../../../..'),collectorPath),
      input:{release:fixture.release,policy_id:policy.id},
      operations:['read_release_qualification'],
    };
    const capture = replay('ready',policy.id);
    const model = modelFromCaptures([capture]);
    return {
      id:config.id,
      policy_id:policy.id,
      config,
      summary:capture.summary,
      request_count:capture.requests.length,
      observation_count:capture.observations.length,
      control_ids:model.evidence.map(item=>item.control_id),
      evidence_count:model.evidence.length,
      _capture:capture,
    };
  });
  const model = modelFromCaptures(integrations.map(integration=>integration._capture));
  for (const integration of integrations) delete integration._capture;
  const scenarios = scenarioNames.map(scenario => ({scenario,assessment:productShip.evaluate(model,scenario)}));
  return {
    ...model,
    provenance: provenanceRunner.collectProvenance(),
    integrations,
    scenarios,
    assessments:Object.fromEntries(scenarios.map(item=>[item.scenario,item.assessment])),
    notes:[
      'Synthetic broker replay; no external system was queried.',
      'The policy tree is descriptive input for this demonstration; native hierarchy enforcement is not claimed.',
      'Controls are proposed examples and do not assert legal or compliance certification.',
      'The nine integration entries are logical source-domain instances of one hosted collector program.',
    ],
  };
}

if (require.main === module) {
  const outputIndex = process.argv.indexOf('--output');
  const target = outputIndex >= 0 ? process.argv[outputIndex + 1] : process.argv[2];
  if (outputIndex >= 0 && !target) throw new Error('output_path_required');
  const output = JSON.stringify(run(), null, 2) + '\n';
  if (target && target !== '-') fs.writeFileSync(path.resolve(target), output, {encoding:'utf8',flag:'w'});
  else process.stdout.write(output);
}
module.exports = {run,replay,modelFromCaptures,providerSnapshot};
