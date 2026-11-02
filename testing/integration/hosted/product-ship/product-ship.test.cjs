const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const {spawnSync}=require('node:child_process');
const dir=__dirname;
const fixture=JSON.parse(fs.readFileSync(path.join(dir,'product-ship.json'),'utf8'));
require('./product-ship.js');
const evaluator=globalThis.IpeProductShip;
const {run,replay,modelFromCaptures}=require('./runner.cjs');

function clone(value) { return JSON.parse(JSON.stringify(value)); }
function firstEvidence(model) { return model.evidence[0]; }

test('runner emits all five named release scenarios from SDK collector observations',()=>{
  const output=run();
  assert.equal(output.schema,'ipe.product-ship-demo.v1');
  assert.equal(output.as_of,fixture.as_of);
  assert.equal(output.integrations.length,9);
  assert.deepEqual(output.scenarios.map(item=>item.scenario),['ready','blocked','missing','expired','wrong-artifact']);
  assert.deepEqual(output.scenarios.map(item=>item.assessment.decision),['allow','deny','indeterminate','indeterminate','indeterminate']);
  assert.ok(output.integrations.every(item=>item.config.program.endsWith('/collector.js')));
  assert.ok(output.integrations.every(item=>item.config.operations.includes('read_release_qualification')));
  assert.ok(output.integrations.every(item=>item.evidence_count===2));
  assert.equal(output.policy.children.length,9);
  assert.equal(output.policies.flatMap(policy=>policy.controls).length,18);
  assert.equal(output.evidence.length,18);
  assert.equal(output.integrations[0].request_count,1);
});

test('each logical source integration runs the same SDK program with a policy-specific input and control output',()=>{
  const output=run();
  const expected=new Map(fixture.policies.map(policy=>[policy.id,policy.controls.map(control=>control.id)]));
  for (const integration of output.integrations) {
    assert.equal(integration.config.input.policy_id,integration.policy_id);
    assert.deepEqual(integration.control_ids,expected.get(integration.policy_id));
    assert.equal(integration.summary.controls_observed,2);
  }
  const policyId=fixture.policies[0].id;
  const capture=replay('ready',policyId,{includeExtraData:true});
  const facts=capture.observations.filter(item=>item.model.schema_version===1&&item.model.entity.kind==='fact');
  assert.deepEqual(facts.map(item=>item.data.control_id),expected.get(policyId));
  const rootObservation=capture.observations.find(item=>item.model.schema_version===2&&item.model.entity.record.id.local_id==='product-ship');
  assert.equal(rootObservation.model.entity.record.requirements.length,fixture.policy.children.length);
  assert.deepEqual(rootObservation.model.entity.record.requirements.map(item=>JSON.parse(item.specification).child_policy_id),fixture.policy.children);
});

test('collector maps successful broker response to receipt-bound governance records and expiring evidence',()=>{
  const capture=replay('ready');
  assert.equal(capture.summary.status,'succeeded');
  assert.equal(capture.summary.publication,false);
  assert.equal(capture.requests.length,1);
  const governance=capture.observations.filter(item=>item.model.schema_version===2);
  const facts=capture.observations.filter(item=>item.model.schema_version===1&&item.model.entity.kind==='fact');
  assert.equal(governance.length,29);
  assert.equal(facts.length,18);
  assert.ok(governance.some(item=>item.model.entity.record.type==='principal'));
  assert.ok(governance.some(item=>item.model.entity.record.id.local_id==='product-ship'));
  assert.ok(governance.some(item=>item.model.entity.record.type==='control'));
  for (const observation of capture.observations) assert.deepEqual(observation.receipts,['fixture:qualification:'+fixture.release.id]);
  for (const fact of facts) {
    assert.equal(fact.data.expires_at,'2026-11-02T12:30:00Z');
    assert.equal(fact.data.release_id,fixture.release.id);
    assert.equal(fact.model.entity.value,'met');
  }
});

test('denied broker read yields explicit unknown and emits no evidence',()=>{
  const capture=replay('denied');
  assert.equal(capture.summary.status,'unknown');
  assert.equal(capture.summary.reason,'provider_read_failed');
  assert.deepEqual(capture.observations,[]);
});

test('wrong-artifact provider identity remains in emitted evidence and evaluates unknown',()=>{
  const model=modelFromCaptures([replay('wrong-artifact')]);
  assert.notEqual(model.evidence[0].artifact,model.release.artifact);
  const assessment=evaluator.evaluate(model);
  assert.equal(assessment.checks[0].status,'unknown');
  assert.match(assessment.checks[0].reason,/artifact does not match/);
  assert.equal(assessment.decision,'indeterminate');
});

test('assessment uses fixed as_of and rejects release, environment, source, future, and duplicate identity evidence',()=>{
  const mutations=[
    item=>{item.release_id='other-release';},
    item=>{item.environment='staging';},
    item=>{item.source='other-source';},
    item=>{item.observed_at='2026-11-02T12:00:01Z';},
    item=>{item.expires_at='2026-11-02T12:00:00Z';},
  ];
  for (const mutate of mutations) {
    const model=clone(fixture); mutate(firstEvidence(model));
    const result=evaluator.evaluate(model);
    assert.equal(result.checks[0].status,'unknown');
    assert.equal(result.decision,'indeterminate');
  }
  const duplicate=clone(fixture);duplicate.evidence.push({...duplicate.evidence[0],status:'not_met'});
  assert.equal(evaluator.evaluate(duplicate).checks[0].status,'unknown');
});

test('central child links and not_met precedence fail closed when policy configuration or other evidence is missing',()=>{
  const missingChild=clone(fixture);missingChild.policies=missingChild.policies.slice(1);
  assert.equal(evaluator.evaluate(missingChild).decision,'indeterminate');
  const emptyTree=clone(fixture);emptyTree.policy.children=[];
  assert.equal(evaluator.evaluate(emptyTree).decision,'indeterminate');
  const blockedAndMissing=clone(fixture);
  blockedAndMissing.evidence[0].status='not_met';blockedAndMissing.evidence.splice(1,1);
  assert.equal(evaluator.evaluate(blockedAndMissing).decision,'deny');
  assert.equal(evaluator.evaluate(clone(fixture),'blocked').decision,'deny');
  assert.equal(evaluator.evaluate(clone(fixture),'missing').decision,'indeterminate');
  assert.equal(evaluator.evaluate(clone(fixture),'expired').decision,'indeterminate');
});

test('CLI writes UI-ready model and assessments to a requested target path',()=>{
  const target=path.join(os.tmpdir(),'ipe-product-ship-'+process.pid+'.json');
  const result=spawnSync(process.execPath,[path.join(dir,'runner.cjs'),'--output',target],{encoding:'utf8'});
  try {
    assert.equal(result.status,0,result.stderr);
    const output=JSON.parse(fs.readFileSync(target,'utf8'));
    assert.equal(output.schema,fixture.schema);
    assert.equal(output.assessments.ready.decision,'allow');
    assert.equal(output.assessments.blocked.decision,'deny');
  } finally { fs.rmSync(target,{force:true}); }
});
