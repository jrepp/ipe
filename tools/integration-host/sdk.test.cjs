const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const path=require('node:path');
const root=path.resolve(__dirname,'../..');
const sdk=fs.readFileSync(path.join(root,'crates/ipe-integration/sdk/index.js'),'utf8');
function evaluate(code) { return JSON.parse(vm.runInNewContext(sdk+'\nJSON.stringify('+code+')',{}, {timeout:1000})); }
test('operation keys preserve v1 association identity and record envelopes',()=>{
 const out=evaluate(`(()=>{
  const operation={namespace:'github',resource:'fixture/repo',operation:'github_actions.run_job',local_id:'.github/workflows/docs.yml#publish'};
  const oldKey=ipe.controlKey(operation),newKey=ipe.operationKey(operation);
  const record=ipe.observed(['native'],'workflow','provider_reported').emit({kind:'association',subject:newKey,predicate:'ipe.control_execution',target:'execution:attempt2',context:{execution:'attempt2',revision:'abc'},authority:'github',resolution:'resolved'});
  return {oldKey,newKey,record};
 })()`);
 assert.equal(out.newKey,'control:["github","fixture/repo","github_actions.run_job",".github/workflows/docs.yml#publish"]');
 assert.equal(out.oldKey,out.newKey);
 assert.equal(out.record.model.schema_version,1);
 assert.equal(out.record.model.entity.subject,out.oldKey);
 assert.equal(out.record.model.entity.kind,'association');
 assert.deepEqual(out.record.receipts,['native']);
});
test('task plan bounds work, deduplicates, resumes responses, and accounts for dropped gaps',()=>{
 const out=evaluate(`(()=>{
  let plan=ipe.tasks(null,{queue:1,tasks:1,gaps:1});
  plan.enqueue({type:'read'},'one');plan.enqueue({type:'read'},'one');
  plan.enqueue({type:'other'},'two');plan.enqueue({type:'other'},'three');
  const task=plan.next(),request=plan.request(task,'sample',{});
  plan=ipe.tasks(JSON.parse(JSON.stringify(plan.state)),{queue:1,tasks:1,gaps:1});
  const received=plan.receive([{id:request.id,operation:'sample',code:'provider_not_found'}]);
  plan.enqueue({type:'other'},'two');const exhausted=plan.next();
  return {request,received,exhausted,state:plan.state,progress:plan.progress('complete')};
 })()`);
 assert.equal(out.request.id,'task_1');assert.equal(out.received.result.code,'provider_not_found');
 assert.equal(out.exhausted,null);assert.equal(out.state.queue.length,0);assert.equal(out.state.droppedGaps,2);
 assert.equal(out.progress.completed,1);assert.equal(out.progress.gaps,3);
 assert.throws(()=>evaluate(`ipe.tasks(null,{queue:65})`));
 assert.throws(()=>evaluate(`(()=>{const p=ipe.tasks();p.request({},'sample',{});p.next();})()`));
 assert.throws(()=>evaluate(`(()=>{const p=ipe.tasks();p.request({},'sample',{});p.receive([]);})()`));
});
test('read-bound builders preserve receipts, explicit claim, and raw data',()=>{
 const out=evaluate(`ipe.readResult([{id:'r',operation:'sample',receipt:'native',data:[{x:1}],pagination:'single_document'}],'r').observations('scope','inferred').emit({kind:'evidence'},{raw:1})`);
 assert.deepEqual(out,{model:{schema_version:1,scope:'scope',claim:'inferred',entity:{kind:'evidence'}},data:{raw:1},receipts:['native']});
 assert.deepEqual(evaluate(`ipe.readResult([{id:'r',operation:'sample',code:'denied'}],'r')`),{id:'r',operation:'sample',code:'denied'});
 assert.throws(()=>evaluate(`ipe.observed([],'scope','declared')`));
});
test('projection preserves unknown keys without prototype assignment and batches do not lose failed work',()=>{
 assert.deepEqual(evaluate(`ipe.project(JSON.parse('{"name":"x","__proto__":{"bad":true},"jobs":{}}'),['name'],['jobs'])`),{name:'x',unmodeled:{['__proto__']:{bad:true}}});
 assert.deepEqual(evaluate(`(()=>{const items=[1,2,3];try{ipe.emitBatch(items,()=>{throw Error('failed')})}catch{}const batch=ipe.emitBatch(items,x=>x*2,2);return {items,batch};})()`),{items:[3],batch:[2,4]});
 assert.equal(evaluate(`ipe.elapsed('2026-01-01T00:00:00Z','2026-01-01T00:00:02Z')`),2000);
 assert.equal(evaluate(`ipe.elapsed('bad','2026-01-01T00:00:02Z')`),null);
});
const {replay}=require('./replay.cjs');
const collector=fs.readFileSync(path.join(root,'testing/integration/hosted/github-actions.js'),'utf8');
test('SDK collector preserves baseline models, receipts, coverage, and read plan across fresh steps',()=>{
 const expected=JSON.parse(fs.readFileSync(path.join(__dirname,'fixtures/github-replay.json'),'utf8'));
 const actual=replay(sdk,collector);assert.ok(actual.progress.length>0);
 delete actual.progress;const links=actual.observations.filter(o=>o.model.entity.kind==='association');assert.equal(links.length,1);assert.equal(links[0].model.entity.resolution,'candidate');assert.equal(links[0].model.entity.context.execution,'execution:["github","fixture/repo","1",1,null,null]');actual.observations=actual.observations.filter(o=>o.model.entity.kind!=='association');assert.equal(actual.summary.dropped_gaps,0);delete actual.summary.dropped_gaps;
 assert.deepEqual(actual,expected);
});
test('collector preserves failed root reads and incomplete sample reporting',()=>{
 assert.throws(()=>replay(sdk,collector,{denied:true}),/root_unavailable/);
 const out=replay(sdk,collector,{truncated:true});
 assert.ok(out.summary.gaps.some(g=>g.code==='source_sample_truncated'));
 assert.equal(out.summary.coverage,'partial');assert.equal(out.summary.publication,false);
});
