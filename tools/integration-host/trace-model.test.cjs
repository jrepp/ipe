const {test}=require('node:test');
const assert=require('node:assert/strict');
const {buildTraceModel,buildDomainFocus}=require('./trace-model.js');
const source=(revision='a')=>({definition:{namespace:'github',resource:'org/repo',path:'ci.yml'},revision,pointer:''});
const control={namespace:'github',resource:'org/repo',operation:'run_job',local_id:'ci.yml#build'};
const read=(receipt='receipt',extra={})=>({step:1,request_id:'read-1',operation:'read_definition',parameters:{path:'ci.yml'},status:'observed',receipt,...extra});
const record=(kind,key,entity,extra={})=>({kind,key,revision_id:'revision:'+key,receipts:['receipt'],model:{entity:{kind,...entity}},...extra});
const emission=(record,step=2)=>({step,kind:record.kind,key:record.key,revision_id:record.revision_id,receipts:record.receipts});
function fixture(){
 const definition=record('definition','definition:ci',{source:source(),title:'CI'});
 const constraint=record('constraint','constraint:build',{source:source(),title:'Build',control,dependencies:[]});
 const fact=record('fact','fact:build',{subject:'attempt:1',predicate:'status',value:'success'});
 return {id:1,status:'succeeded',records:[definition,constraint,fact],evidence:[read()],trace:{schema_version:1,emissions:[definition,constraint,fact].map(r=>emission(r)),omitted:0}};
}
test('trace links exact read receipts and model sources without changing run data',()=>{
 const run=fixture(),before=JSON.stringify(run),model=buildTraceModel(run);
 assert.equal(JSON.stringify(run),before);assert.equal(model.legacy,false);
 assert.deepEqual(model.counts,{reads:1,models:2,facts:1,nodes:4,edges:4,issues:0});
 assert.equal(model.edges.filter(e=>e.kind==='receipt').length,3);
 assert.ok(model.edges.some(e=>e.kind==='model'&&e.source==='record:0'&&e.target==='record:1'&&e.status==='resolved'));
 assert.equal(model.nodes.find(n=>n.lane==='fact').step,2);
 assert.deepEqual(model.omitted,{nodes:0,edges:0,emissions:0});
});
test('old runs use their own receipts and records without inventing emission steps',()=>{
 const run=fixture();delete run.trace;delete run.evidence[0].step;
 run.current_records=[record('fact','unrelated',{predicate:'new'})];
 const model=buildTraceModel(run);
 assert.equal(model.legacy,true);assert.equal(model.nodes.length,4);
 assert.ok(model.nodes.every(n=>n.step===undefined));
 assert.ok(!model.nodes.some(n=>n.label==='new'));
 assert.equal(model.issues.length,0);
});
test('failed runs retain discarded emissions and failed reads; running emissions remain staged',()=>{
 const run=fixture();run.status='failed';run.records=[];
 run.evidence.push(read('',{request_id:'read-2',step:3,status:'unavailable',operation:'next',code:'permission_denied'}));
 let model=buildTraceModel(run);
 assert.equal(model.nodes.filter(n=>n.status==='discarded').length,3);
 assert.equal(model.edges.filter(e=>e.kind==='receipt').length,3);
 assert.ok(model.issues.some(i=>i.code==='read_failed'&&i.nodeId==='read:1'));
 assert.ok(!model.issues.some(i=>i.code==='read_without_output'));
 run.status='running';run.evidence[1].status='pending';
 model=buildTraceModel(run);
 assert.equal(model.nodes.filter(n=>n.status==='staged').length,3);
 assert.ok(!model.issues.some(i=>i.code==='read_failed'));
});
test('matching kind and key with a different revision never joins an emission to a record',()=>{
 const run=fixture();run.trace.emissions[1].revision_id='different';
 const model=buildTraceModel(run);
 assert.equal(model.nodes.find(n=>n.id==='record:1').step,undefined);
 assert.equal(model.nodes.find(n=>n.id==='emission:1').status,'unavailable');
 assert.ok(model.issues.some(i=>i.code==='missing_emission_trace'&&i.nodeId==='record:1'));
});
test('missing and ambiguous receipts stay gaps; traversal reads are not violations',()=>{
 const run=fixture();run.records[2].receipts=['missing'];run.trace.emissions[2].receipts=['missing'];
 run.evidence.push(read('traversal',{operation:'resolve_revision'}));
 const model=buildTraceModel(run);
 assert.ok(model.issues.some(i=>i.code==='missing_receipt'&&i.nodeId==='record:2'));
 assert.ok(model.issues.some(i=>i.code==='read_without_output'&&i.message==='No emitted record (may be a traversal read)'));
 assert.equal(model.edges.filter(e=>e.kind==='receipt').length,2);
 run.evidence.push(read());
 const ambiguous=buildTraceModel(run);
 assert.ok(ambiguous.issues.some(i=>i.code==='ambiguous_receipt'));
 assert.equal(ambiguous.edges.filter(e=>e.kind==='receipt').length,0);
});
test('model references require exact source revisions and dependency references remain candidates',()=>{
 const run=fixture();
 const invoke=record('invocation','call',{parent:source(),callee:source('other'),requested:'action@other',resolution:'candidate',bindings:[]});
 const dependent=record('constraint','dependent',{source:source(),control:{...control,local_id:'publish'},title:'Publish',dependencies:[control]});
 run.records.push(invoke,dependent);run.trace.emissions.push(emission(invoke),emission(dependent));
 let model=buildTraceModel(run);
 assert.ok(model.issues.some(i=>i.code==='unresolved_model_reference'&&i.nodeId==='record:3'));
 assert.ok(model.edges.some(e=>e.source==='record:1'&&e.target==='record:4'&&e.status==='candidate'));
 const other=record('definition','other',{source:source('other'),title:'Other'});
 run.records.push(other);run.trace.emissions.push(emission(other));model=buildTraceModel(run);
 assert.ok(model.edges.some(e=>e.source==='record:5'&&e.target==='record:3'&&e.status==='candidate'));
});
test('display caps do not turn retained receipts or sources into gaps',()=>{
 const run=fixture();
 for(let i=0;i<200;i++){
   const r=record('fact','fact:'+i,{predicate:'field'+i});
   run.records.push(r);run.trace.emissions.push(emission(r,3));
 }
 const model=buildTraceModel(run);
 assert.equal(model.nodes.length,180);assert.equal(model.counts.nodes,204);
 assert.equal(model.omitted.nodes,24);assert.ok(model.omitted.edges>0);
 assert.equal(model.issues.length,0);
 assert.ok(model.nodes.some(n=>n.lane==='read'));assert.ok(model.nodes.some(n=>n.lane==='model'));
 assert.ok(model.edges.every(e=>model.nodes.some(n=>n.id===e.source)&&model.nodes.some(n=>n.id===e.target)));
});
test('native trace omissions suppress claims that no output was emitted',()=>{
 const model=buildTraceModel({status:'succeeded',records:[],evidence:[read()],trace:{schema_version:1,emissions:[],omitted:3}});
 assert.equal(model.omitted.emissions,3);assert.ok(!model.issues.some(i=>i.code==='read_without_output'));
 const pending=buildTraceModel({status:'running',evidence:[read('',{status:'pending'})]});
 assert.equal(pending.issues.length,0);
});
test('standalone evidence locators do not imply missing interface definition records',()=>{
 const excerpt=record('evidence','source-excerpt',{source:{...source(),definition:{...source().definition,path:'src/main.js'}},method:'source_excerpt',description:'Captured implementation text'});
 const model=buildTraceModel({status:'succeeded',records:[excerpt],evidence:[read()],trace:{schema_version:1,emissions:[emission(excerpt)],omitted:0}});
 assert.equal(model.issues.length,0);assert.equal(model.edges.length,1);
 assert.equal(model.edges[0].kind,'receipt');
});
test('unknown trace versions never supply trusted emission steps',()=>{
 const run=fixture();run.trace.schema_version=2;run.trace.emissions[0].step=99;
 const model=buildTraceModel(run);
 assert.equal(model.legacy,true);
 assert.ok(model.nodes.filter(n=>n.lane!=='read').every(n=>n.step===undefined));
 assert.ok(!model.nodes.some(n=>n.id.startsWith('emission:')));
});
test('execution overlay links exact outcomes to facts and preserves candidate declaration status',()=>{
 const run=fixture(),execution={namespace:'github',resource:'org/repo',run:'42',attempt:2,job:'build',step:null};
 run.records[2].model.entity.execution=execution;
 const outcome=record('outcome','result',{execution,definition:source(),title:'Build execution',declaration_candidates:[control],correlation:'candidate'});
 run.records.push(outcome);run.trace.emissions.push(emission(outcome));
 const model=buildTraceModel(run);
 assert.ok(model.edges.some(e=>e.source==='record:0'&&e.target==='record:3'&&e.label==='Execution definition'&&e.status==='resolved'));
 assert.ok(model.edges.some(e=>e.source==='record:1'&&e.target==='record:3'&&e.status==='candidate'));
 assert.ok(model.edges.some(e=>e.source==='record:3'&&e.target==='record:2'&&e.label==='Same execution'&&e.status==='resolved'));
 assert.equal(model.nodes.find(n=>n.id==='record:2').label,'Build execution · status');
 assert.equal(model.nodes.find(n=>n.id==='record:2').detail.record.model.entity.predicate,'status');
 assert.equal(model.issues.length,0);
});
test('missing and ambiguous outcomes never produce an invented execution link',()=>{
 const run=fixture(),execution={namespace:'github',resource:'org/repo',run:'42',attempt:2,job:'build',step:null};
 run.records[2].model.entity.execution=execution;
 let model=buildTraceModel(run);
 assert.ok(model.issues.some(i=>i.nodeId==='record:2'&&i.code==='unresolved_model_reference'));
 const outcome=record('outcome','attempt-one',{execution:{...execution,attempt:1},definition:source(),declaration_candidates:[control],correlation:'candidate'});
 run.records.push(outcome);run.trace.emissions.push(emission(outcome));model=buildTraceModel(run);
 assert.ok(!model.edges.some(e=>e.target==='record:2'&&e.kind==='model'));
 outcome.model.entity.execution.attempt=2;
 const duplicate=structuredClone(outcome);duplicate.key='duplicate';duplicate.revision_id='duplicate';
 run.records.push(duplicate);run.trace.emissions.push(emission(duplicate));model=buildTraceModel(run);
 assert.ok(model.issues.some(i=>i.nodeId==='record:2'&&/More than one retained outcome/.test(i.message)));
 assert.ok(!model.edges.some(e=>e.target==='record:2'&&e.kind==='model'));
});
test('outcome candidates require their source revision and unknown correlation stays explicit',()=>{
 const run=fixture(),execution={namespace:'github',resource:'org/repo',run:'42',attempt:1,job:null,step:null};
 const outcome=record('outcome','workflow',{execution,definition:source('other'),declaration_candidates:[control],correlation:'candidate'});
 run.records.push(outcome);run.trace.emissions.push(emission(outcome));
 let model=buildTraceModel(run);
 assert.ok(!model.edges.some(e=>e.target==='record:3'&&e.kind==='model'));
 assert.ok(model.issues.some(i=>i.nodeId==='record:3'&&/declaration candidate and its source revision/.test(i.message)));
 outcome.model.entity.definition=source();outcome.model.entity.declaration_candidates=[];outcome.model.entity.correlation='unknown';
 model=buildTraceModel(run);
 assert.ok(model.issues.some(i=>i.nodeId==='record:3'&&i.code==='uncertain_model_reference'));
 assert.equal(model.nodes.find(n=>n.id==='record:3').status,'committed');
});
function domainFixture(){
 const run=fixture(),execution={namespace:'github',resource:'org/repo',run:'42',attempt:2,job:'build',step:null};
 Object.assign(run.records[1].model.entity,{actor_context:'unobserved',enforcer:'github_actions',applicability:{dialect:'github',expression:null,default_behavior:'unknown'},obligations:[{id:'approval',description:'Approval evidence',requirement:{kind:'opaque',reason:'Unknown approval policy'}}]});
 run.records[2].model.entity.execution=execution;
 const outcome=record('outcome','result',{execution,definition:source(),title:'Build',status:'completed',conclusion:'success',declaration_candidates:[control],correlation:'candidate'});
 run.records.push(outcome);run.trace.emissions.push(emission(outcome));return run;
}
test('domain focus separates declared requirements from successful external execution',()=>{
 const run=domainFixture(),before=JSON.stringify(run),focus=buildDomainFocus(run);
 assert.equal(focus.mode,'operation');assert.equal(focus.operation.id,'record:1');assert.equal(focus.execution.id,'record:3');
 assert.deepEqual(focus.operation.operation,control);assert.equal(focus.operation.declaredEnforcer,'github_actions');
 assert.equal(focus.operation.requirements[0].id,'approval');assert.equal(focus.operation.requirements[0].fulfillment,'unknown');
 assert.equal(focus.operation.requirements[0].specification.kind,'opaque');
 assert.equal(focus.edges.length,2);
 assert.ok(focus.edges.some(edge=>edge.source==='record:1'&&edge.target==='record:3'&&edge.status==='candidate'));
 assert.ok(focus.edges.some(edge=>edge.source==='record:3'&&edge.target==='record:2'&&edge.status==='resolved'));
 assert.equal(focus.facts[0].id,'record:2');assert.ok(focus.support.nodes.some(node=>node.kind==='read'));
 assert.ok(focus.support.nodes.some(node=>node.kind==='definition'));assert.equal(JSON.stringify(run),before);
});
test('domain focus never pairs missing operation executions or unrelated facts',()=>{
 const run=fixture(),focus=buildDomainFocus(run);
 assert.equal(focus.execution,null);assert.deepEqual(focus.facts,[]);assert.equal(focus.unmapped.facts,1);
 const unmapped=buildDomainFocus(run,{operationId:'__unmapped__'});
 assert.equal(unmapped.operation,null);assert.equal(unmapped.execution,null);assert.equal(unmapped.facts[0].id,'record:2');assert.deepEqual(unmapped.edges,[]);
});
test('unmapped executions retain only their exact facts while orphan facts stay separate',()=>{
 const run=domainFixture(),outcome=run.records[3];outcome.model.entity.declaration_candidates=[];outcome.model.entity.correlation='unknown';
 const orphan=record('fact','orphan',{predicate:'status',value:'success'});run.records.push(orphan);run.trace.emissions.push(emission(orphan));
 const focus=buildDomainFocus(run,{operationId:'__unmapped__'});
 assert.equal(focus.unmapped.executions,1);assert.equal(focus.unmapped.facts,1);
 assert.equal(focus.execution.id,'record:3');assert.deepEqual(focus.facts.map(node=>node.id),['record:2']);
 const separate=buildDomainFocus(run,{operationId:'__unmapped__',executionId:'__unlinked_facts__'});
 assert.equal(separate.execution,null);assert.deepEqual(separate.facts.map(node=>node.id),['record:4']);assert.deepEqual(separate.edges,[]);
});
test('domain facts page in groups of six without diagnostic cap causing false orphans',()=>{
 const run=domainFixture(),execution=run.records[3].model.entity.execution;
 for(let i=0;i<200;i++){const fact=record('fact','extra:'+i,{predicate:'field:'+i,execution});run.records.push(fact);run.trace.emissions.push(emission(fact));}
 const focus=buildDomainFocus(run,{factPage:33});
 assert.equal(focus.allFactCount,201);assert.equal(focus.pageCount,34);assert.equal(focus.facts.length,3);
 assert.equal(focus.unmapped.facts,0);assert.equal(focus.omitted.nodes,0);
 assert.ok(focus.edges.length<=7);assert.equal(focus.operation.requirements[0].fulfillment,'unknown');
 assert.equal(buildDomainFocus(run,{factPage:999}).page,33);
});
test('failed staged facts and legacy runs remain inspectable without invented operations',()=>{
 const run=domainFixture();run.status='failed';run.records=[];
 let focus=buildDomainFocus(run);
 assert.equal(focus.mode,'unmapped');assert.equal(focus.operation,null);
 focus=buildDomainFocus(run,{operationId:'__unmapped__',executionId:'__unlinked_facts__'});
 assert.equal(focus.facts.length,1);assert.equal(focus.facts[0].status,'discarded');assert.equal(focus.execution,null);
 assert.ok(focus.support.nodes.some(node=>node.lane==='read'));
 const old=domainFixture();delete old.trace;focus=buildDomainFocus(old);
 assert.equal(focus.legacy,true);assert.equal(focus.operation.step,undefined);assert.equal(focus.execution.step,undefined);
 assert.equal(focus.facts[0].step,undefined);assert.equal(focus.facts.length,1);
});
