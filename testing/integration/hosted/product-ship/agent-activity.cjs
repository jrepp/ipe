// Deterministic illustrative activity for the product ship policy model.
'use strict';

const crypto = require('node:crypto');
const {createOverviewHistory}=require('./overview-history.cjs');

const SCENARIOS = ['ready', 'blocked', 'missing', 'expired', 'wrong-artifact'];
const AGENTS = [
  {id:'agent-accessibility',name:'Accessibility reviewer'},
  {id:'agent-security',name:'Security reviewer'},
  {id:'agent-legal',name:'Open-source reviewer'},
  {id:'agent-build',name:'Build provenance reviewer'},
  {id:'agent-quality',name:'Test quality reviewer'},
  {id:'agent-operations',name:'Operations reviewer'},
];
const POLICY_AGENT = {
  'ship-accessibility':'agent-accessibility',
  'ship-security':'agent-security',
  'ship-open-source-legal':'agent-legal',
  'ship-build-provenance':'agent-build',
  'ship-test-quality':'agent-quality',
  'ship-privacy':'agent-security',
  'ship-change-control':'agent-operations',
  'ship-reliability':'agent-operations',
  'ship-operations':'agent-operations',
};
const PREPARATION = {input_tokens:12000,output_tokens:900,elapsed_ms:18000,work_ms:18000};
const BASELINE = {input_tokens:78000,output_tokens:10800,elapsed_ms:180000,work_ms:540000};

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

function fingerprint(model, scenario, assessment) {
  const evaluatedModel = {...model, evidence:globalThis.IpeProductShip.simulateScenario(model, scenario).evidence};
  return `sha256:${crypto.createHash('sha256').update(canonical({model:evaluatedModel,scenario,assessment})).digest('hex')}`;
}

function metricDelta(baseline, modeled) {
  if (baseline == null || modeled == null) return null;
  if (!Number.isFinite(baseline) || !Number.isFinite(modeled)) return null;
  if (baseline <= 0 || modeled < 0) return null;
  return {baseline,modeled,saved:baseline-modeled,saved_percent:((baseline-modeled)/baseline)*100};
}

function makeScenario(model, scenario, revision, snapshot, scenarioIndex) {
  const evaluator=globalThis.IpeProductShip;
  const evaluatedEvidence=evaluator.simulateScenario(model,scenario).evidence;
  const assessment=evaluator.evaluate(model,scenario);
  const controls = assessment.checks.filter(check => check.control_id.startsWith('ship.'));
  const queriedIds = controls.map(check => check.control_id);
  const qualifiedIds = new Set(queriedIds.slice(0, 15));
  const checkById = new Map(controls.map(check => [check.control_id,check]));
  const start = Date.parse(model.as_of) + scenarioIndex * 60000;
  const events = [];
  const queryCounts = new Map();
  const availableAt = new Map();
  let cursor = 0;
  const push = (agentId, controlId, action, result = null, duplicate = false) => {
    const check = checkById.get(controlId);
    const matched = check?.evidence;
    const index = cursor++;
    const duration = action === 'query' ? 1400 + (index % 3) * 100 : 700 + (index % 2) * 100;
    const began = Math.max(start + (index % 4) * 100,availableAt.get(agentId) || start);
    availableAt.set(agentId,began + duration);
    const status = 'completed';
    events.push({
      id:`${scenario}-event-${String(index+1).padStart(2,'0')}`,
      agent_id:agentId,
      control_id:controlId,
      action,
      status,
      ...(action === 'qualify' ? {result} : {}),
      started_at:new Date(began).toISOString(),
      ended_at:new Date(began + duration).toISOString(),
      input_tokens:action === 'query' ? (duplicate ? 850 : 1100) : 700,
      output_tokens:action === 'query' ? (duplicate ? 120 : 160) : 110,
      evidence_receipt:matched?.receipt || null,
      model_revision:revision,
      evidence_snapshot:snapshot,
      release_id:model.release.id,
      artifact:model.release.artifact,
      environment:model.release.environment,
    });
  };

  // Every control is queried. One qualified control is queried twice to make
  // call volume distinct from the count of unique controls and agents.
  for (let i=0;i<queriedIds.length;i++) {
    const controlId=queriedIds[i];
    const agentId=POLICY_AGENT[checkById.get(controlId).policy_id] || AGENTS[i % AGENTS.length].id;
    push(agentId,controlId,'query');
    queryCounts.set(controlId,1);
  }
  if (queriedIds.length) {
    const repeated=queriedIds[0];
    push('agent-operations',repeated,'query',null,true);
    queryCounts.set(repeated,2);
  }
  for (let i=0;i<queriedIds.length;i++) {
    const controlId=queriedIds[i];
    if (!qualifiedIds.has(controlId)) continue;
    const agentId=POLICY_AGENT[checkById.get(controlId).policy_id] || AGENTS[i % AGENTS.length].id;
    push(agentId,controlId,'qualify',checkById.get(controlId).status);
  }

  const unique = (values) => [...new Set(values)];
  const callInput = events.reduce((sum,event)=>sum+event.input_tokens,0);
  const callOutput = events.reduce((sum,event)=>sum+event.output_tokens,0);
  const times = events.flatMap(event=>[Date.parse(event.started_at),Date.parse(event.ended_at)]);
  const queried = unique(events.filter(event=>event.action==='query').map(event=>event.control_id));
  const qualified = events.filter(event=>event.action==='qualify');
  const resultByControl = new Map(qualified.map(event=>[event.control_id,event.result]));
  const qualifiedMet = new Set(qualified.filter(event=>event.result==='met' && event.evidence_receipt &&
    event.release_id===model.release.id && event.artifact===model.release.artifact &&
    event.environment===model.release.environment && checkById.get(event.control_id)?.status==='met').map(event=>event.control_id));
  const execution = {
    calls:events.length,
    query_calls:events.filter(event=>event.action==='query').length,
    qualification_attempts:qualified.length,
    input_tokens:callInput,
    output_tokens:callOutput,
    total_tokens:callInput+callOutput,
    elapsed_ms:Math.max(...times)-Math.min(...times),
    work_ms:events.reduce((sum,event)=>sum+(Date.parse(event.ended_at)-Date.parse(event.started_at)),0),
  };
  const agentActivity=AGENTS.map(agent=>{
    const own=events.filter(event=>event.agent_id===agent.id);
    const ownQueries=own.filter(event=>event.action==='query');
    const ownQualifications=own.filter(event=>event.action==='qualify');
    const ownTimes=own.flatMap(event=>[Date.parse(event.started_at),Date.parse(event.ended_at)]);
    return {
      ...agent,
      queried_steps:unique(ownQueries.map(event=>event.control_id)),
      qualified_steps:unique(ownQualifications.filter(event=>event.result==='met'&&event.evidence_receipt).map(event=>event.control_id)),
      qualification_attempts:ownQualifications.map(event=>event.control_id),
      query_count:ownQueries.length,
      qualification_count:unique(ownQualifications.filter(event=>event.result==='met'&&event.evidence_receipt).map(event=>event.control_id)).length,
      qualification_attempt_count:ownQualifications.length,
      elapsed_ms:own.length?Math.max(...ownTimes)-Math.min(...ownTimes):0,
      input_tokens:own.reduce((sum,event)=>sum+event.input_tokens,0),
      output_tokens:own.reduce((sum,event)=>sum+event.output_tokens,0),
    result:ownQualifications.length ? ownQualifications.map(event=>event.result) : [],
      evidence_receipts:unique(own.map(event=>event.evidence_receipt).filter(Boolean)),
      source_revision:revision,
      duration_ms:own.reduce((sum,event)=>sum+(Date.parse(event.ended_at)-Date.parse(event.started_at)),0),
    };
  });
  const baseline = {...BASELINE,total_tokens:BASELINE.input_tokens+BASELINE.output_tokens};
  const coldRun={
    ...execution,
    input_tokens:execution.input_tokens+PREPARATION.input_tokens,
    output_tokens:execution.output_tokens+PREPARATION.output_tokens,
    total_tokens:execution.total_tokens+PREPARATION.input_tokens+PREPARATION.output_tokens,
    elapsed_ms:execution.elapsed_ms+PREPARATION.elapsed_ms,
    work_ms:execution.work_ms+PREPARATION.work_ms,
    model_preparation:{...PREPARATION,counted_once:true},
  };
  const comparison = {
    label:'Illustrative paired raw-source baseline',
    basis:'Same scenario, 18 controls, release scope, and qualification success criteria.',
    baseline,
    modeled:coldRun,
    savings:{
      total_tokens:metricDelta(baseline.input_tokens+baseline.output_tokens,coldRun.total_tokens),
      elapsed_ms:metricDelta(baseline.elapsed_ms,coldRun.elapsed_ms),
      work_ms:metricDelta(baseline.work_ms,coldRun.work_ms),
    },
  };
  return {
    scenario,
    fingerprint:fingerprint(model,scenario,assessment),
    scope:{
      release_id:model.release.id,
      artifact:model.release.artifact,
      environment:model.release.environment,
      as_of:model.as_of,
      model_revision:revision,
      evidence_snapshot:snapshot,
      evidence_manifest:{as_of:model.as_of,evidence:evaluatedEvidence.map(item=>({
        control_id:item.control_id,release_id:item.release_id,artifact:item.artifact,
        environment:item.environment,source:item.source,status:item.status,
        observed_at:item.observed_at,expires_at:item.expires_at,receipt:item.receipt,
      }))},
      policy_manifest:JSON.parse(JSON.stringify({policy:model.policy,policies:model.policies})),
    },
    queried_control_ids:queried,
    query_counts:Object.fromEntries(queryCounts),
    qualification_results:Object.fromEntries([...resultByControl.entries()]),
    agent_activity:agentActivity,
    summary:{
      agent_count:unique(events.map(event=>event.agent_id)).length,
      agents:unique(events.map(event=>event.agent_id)),
      queried_controls:queried.length,
      qualified_controls:qualifiedMet.size,
      qualification_attempts:qualified.length,
      query_only_controls:queried.filter(id=>!qualifiedIds.has(id)).length,
      ...execution,
    },
    comparison,
    events,
  };
}

function createAgentActivity(model) {
  if (!globalThis.IpeProductShip || typeof globalThis.IpeProductShip.evaluate !== 'function') {
    require('./product-ship.js');
  }
  const evaluator=globalThis.IpeProductShip;
  const revision=`model-${crypto.createHash('sha256').update(canonical({policy:model.policy,policies:model.policies})).digest('hex').slice(0,16)}`;
  const scenarios=SCENARIOS.map((scenario,index)=>{
    const snapshot=`evidence-${crypto.createHash('sha256').update(canonical(evaluator.simulateScenario(model,scenario).evidence)).digest('hex').slice(0,16)}`;
    return makeScenario(model,scenario,revision,snapshot,index);
  });
  const runtimeTotals=scenarios.reduce((sum,item)=>{
    for (const key of ['calls','query_calls','qualification_attempts','input_tokens','output_tokens','total_tokens','work_ms']) sum[key]=(sum[key]||0)+item.summary[key];
    sum.elapsed_ms=(sum.elapsed_ms||0)+item.summary.elapsed_ms;
    return sum;
  },{});
  const aggregateModeled={
    ...runtimeTotals,
    input_tokens:runtimeTotals.input_tokens+PREPARATION.input_tokens,
    output_tokens:runtimeTotals.output_tokens+PREPARATION.output_tokens,
    total_tokens:runtimeTotals.total_tokens+PREPARATION.input_tokens+PREPARATION.output_tokens,
    elapsed_ms:runtimeTotals.elapsed_ms+PREPARATION.elapsed_ms,
    work_ms:runtimeTotals.work_ms+PREPARATION.work_ms,
    model_preparation:{...PREPARATION,counted_once:true},
  };
  const rawBaseline={
    input_tokens:BASELINE.input_tokens*SCENARIOS.length,
    output_tokens:BASELINE.output_tokens*SCENARIOS.length,
    total_tokens:(BASELINE.input_tokens+BASELINE.output_tokens)*SCENARIOS.length,
    elapsed_ms:BASELINE.elapsed_ms*SCENARIOS.length,
    work_ms:BASELINE.work_ms*SCENARIOS.length,
  };
  return {
    schema:'ipe.product-ship-agent-activity.v1',
    label:'Illustrative simulated agent activity',
    measured:false,
    units:'fixture tokens and milliseconds; no monetary cost',
    overview_history:createOverviewHistory(model,revision),
    agents:AGENTS,
    model_preparation:{...PREPARATION,counted_once:true},
    aggregate_comparison:{
      basis:'Five paired scenario runs; each pair uses the same controls, release scope, and qualification success criteria. Aggregate elapsed is the sum of run spans plus preparation, excluding gaps between scenarios.',
      baseline:rawBaseline,
      modeled:aggregateModeled,
      savings:{
        total_tokens:metricDelta(rawBaseline.total_tokens,aggregateModeled.total_tokens),
        elapsed_ms:metricDelta(rawBaseline.elapsed_ms,aggregateModeled.elapsed_ms),
        work_ms:metricDelta(rawBaseline.work_ms,aggregateModeled.work_ms),
      },
    },
    scenarios,
    assumptions:[
      'All activity, token, and time values are illustrative fixture values, not measurements or benchmark results.',
      'The paired baseline is simulated raw-source work for the same scenario and success criteria.',
      'Elapsed time is the span from earliest start to latest end, so parallel calls are not added together; work time is summed.',
      'Each selected scenario comparison includes one-time model preparation for a cold run; the five-scenario aggregate includes preparation once total. Retries and cache savings are not modeled.',
      'Qualification results come from the shared product ship evaluator for this scenario and evidence scope.',
    ],
  };
}

module.exports={createAgentActivity,metricDelta};
