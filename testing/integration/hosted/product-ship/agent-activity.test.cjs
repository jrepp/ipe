const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
require('./product-ship.js');
const {createAgentActivity,metricDelta}=require('./agent-activity.cjs');
const model=JSON.parse(fs.readFileSync(path.join(__dirname,'product-ship.json'),'utf8'));

function clone(value) { return JSON.parse(JSON.stringify(value)); }

test('activity records six simulated agents, all controls, repeated query calls, and 15 qualification attempts per scenario',()=>{
  const activity=createAgentActivity(model);
  assert.equal(activity.measured,false);
  assert.match(activity.label,/Illustrative simulated/);
  assert.equal(activity.agents.length,6);
  assert.deepEqual(activity.scenarios.map(item=>item.scenario),['ready','blocked','missing','expired','wrong-artifact']);
  for (const scenario of activity.scenarios) {
    assert.equal(scenario.summary.agent_count,6);
    assert.equal(scenario.summary.queried_controls,18);
    assert.equal(scenario.summary.qualification_attempts,15);
    assert.equal(scenario.summary.query_only_controls,3);
    assert.equal(scenario.summary.query_calls,19);
    assert.equal(scenario.summary.calls,34);
    assert.equal(scenario.queried_control_ids.length,18);
    assert.equal(Object.values(scenario.query_counts).reduce((sum,count)=>sum+count,0),19);
    assert.equal(scenario.query_counts[scenario.queried_control_ids[0]],2);
    assert.equal(scenario.agent_activity.length,6);
    assert.equal(scenario.agent_activity.reduce((sum,item)=>sum+item.qualification_attempt_count,0),15);
    assert.equal(scenario.agent_activity.reduce((sum,item)=>sum+item.qualification_count,0),scenario.summary.qualified_controls);
    assert.ok(scenario.agent_activity.every(item=>item.qualified_steps.length===item.qualification_count));
    for (const agent of scenario.agent_activity) {
      const own=scenario.events.filter(event=>event.agent_id===agent.id).sort((a,b)=>Date.parse(a.started_at)-Date.parse(b.started_at));
      for (let i=1;i<own.length;i++) assert.ok(Date.parse(own[i].started_at)>=Date.parse(own[i-1].ended_at));
    }
  }
});

test('qualification event results match shared evaluator checks and only current met evidence counts as qualified',()=>{
  const activity=createAgentActivity(model);
  const evaluator=globalThis.IpeProductShip;
  for (const scenario of activity.scenarios) {
    const assessment=evaluator.evaluate(model,scenario.scenario);
    const checks=new Map(assessment.checks.map(check=>[check.control_id,check]));
    for (const event of scenario.events.filter(item=>item.action==='qualify')) {
      assert.equal(event.result,checks.get(event.control_id).status);
      assert.equal(event.status,'completed');
      assert.equal(event.release_id,model.release.id);
      assert.equal(event.artifact,model.release.artifact);
      assert.equal(event.environment,model.release.environment);
    }
    const attempted=new Set(scenario.events.filter(event=>event.action==='qualify').map(event=>event.control_id));
    const expected=assessment.checks.filter(check=>attempted.has(check.control_id)&&check.status==='met'&&check.evidence?.receipt&&
      check.evidence.release_id===model.release.id&&check.evidence.artifact===model.release.artifact&&check.evidence.environment===model.release.environment).length;
    assert.equal(scenario.summary.qualified_controls,expected);
  }
});

test('scenario fingerprints and manifests bind activity to evidence, scope, model revision, and scenario',()=>{
  const baseline=createAgentActivity(model);
  const changed=clone(model);
  changed.evidence[0].status='not_met';
  const changedActivity=createAgentActivity(changed);
  const changedRequirement=clone(model);
  changedRequirement.policies[0].controls[0].source='qualification-snapshot-v2';
  const changedRequirementActivity=createAgentActivity(changedRequirement);
  assert.notEqual(baseline.scenarios[0].fingerprint,baseline.scenarios[1].fingerprint);
  assert.notEqual(baseline.scenarios[0].fingerprint,changedActivity.scenarios[0].fingerprint);
  assert.notEqual(baseline.scenarios[0].fingerprint,changedRequirementActivity.scenarios[0].fingerprint);
  assert.notEqual(baseline.scenarios[0].scope.model_revision,changedRequirementActivity.scenarios[0].scope.model_revision);
  assert.notEqual(baseline.scenarios[0].scope.evidence_snapshot,changedActivity.scenarios[0].scope.evidence_snapshot);
  assert.equal(baseline.scenarios[0].scope.policy_manifest.policies.length,model.policies.length);
  assert.equal(baseline.scenarios[0].scope.evidence_manifest.evidence.length,model.evidence.length);
  assert.equal(baseline.scenarios[0].scope.release_id,model.release.id);
  const mutable=clone(model), recorded=createAgentActivity(mutable);
  mutable.policies[0].controls[0].description='Changed after activity capture';
  assert.notEqual(recorded.scenarios[0].scope.policy_manifest.policies[0].controls[0].description,mutable.policies[0].controls[0].description);
});

test('paired comparisons include preparation, guard zero denominators, and count it once in the aggregate',()=>{
  const activity=createAgentActivity(model);
  for (const scenario of activity.scenarios) {
    assert.equal(scenario.comparison.basis.includes('Same scenario'),true);
    assert.equal(scenario.comparison.modeled.model_preparation.counted_once,true);
    assert.equal(scenario.comparison.modeled.input_tokens,scenario.summary.input_tokens+activity.model_preparation.input_tokens);
    assert.equal(scenario.comparison.modeled.elapsed_ms,scenario.summary.elapsed_ms+activity.model_preparation.elapsed_ms);
    assert.equal(scenario.comparison.modeled.total_tokens,scenario.summary.total_tokens+
      activity.model_preparation.input_tokens+activity.model_preparation.output_tokens);
    assert.equal(scenario.comparison.baseline.total_tokens,
      scenario.comparison.baseline.input_tokens+scenario.comparison.baseline.output_tokens);
    assert.ok(scenario.comparison.savings.total_tokens.saved>0);
    assert.ok(scenario.comparison.savings.elapsed_ms.saved>0);
  }
  assert.equal(activity.model_preparation.counted_once,true);
  assert.equal(activity.aggregate_comparison.modeled.model_preparation.counted_once,true);
  assert.equal(activity.aggregate_comparison.modeled.input_tokens,
    activity.scenarios.reduce((sum,item)=>sum+item.summary.input_tokens,0)+activity.model_preparation.input_tokens);
  assert.equal(activity.aggregate_comparison.modeled.output_tokens,
    activity.scenarios.reduce((sum,item)=>sum+item.summary.output_tokens,0)+activity.model_preparation.output_tokens);
  assert.deepEqual(metricDelta(100,120),{baseline:100,modeled:120,saved:-20,saved_percent:-20});
  assert.equal(metricDelta(0,12),null);
  assert.equal(metricDelta(null,12),null);
});

test('overview samples separate evaluation errors from outcomes and pair completed runs',()=>{
  const history=createAgentActivity(model).overview_history;
  assert.equal(history.measured,false);
  assert.equal(history.days.length,7);
  assert.equal(history.scope.release_id,model.release.id);
  for (const [index,day] of history.days.entries()) {
    assert.equal(day.runs,day.allow+day.deny+day.unknown+day.errors);
    assert.equal(day.paired_runs,day.runs-day.errors);
    assert.equal(day.baseline_elapsed_ms,day.paired_runs*180000);
    assert.ok(day.modeled_elapsed_ms>=day.paired_runs*18000);
    assert.ok(day.modeled_input_tokens>=day.paired_runs*12000);
    if(index) assert.equal(Date.parse(day.date)-Date.parse(history.days[index-1].date),86400000);
  }
});
