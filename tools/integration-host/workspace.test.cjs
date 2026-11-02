const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {JSDOM}=require('../../docs/node_modules/jsdom');
const html=fs.readFileSync(__dirname+'/index.html','utf8');
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));
function fixture(){
 const pkg={source:'function step() { return "new <img src=x>"; }',input:{repository:'org/repo',workflow:'ci.yml',revision:'abc'},operations:['read'],output_kinds:['definition','fact'],enabled:true,interval_seconds:300,fact_ttl_seconds:900};
 const record={kind:'definition',key:'definition:one',adapter_digest:'new',run_id:2,observed_at:10,model:{entity:{title:'Build',source:{definition:{resource:'org/repo',path:'ci.yml'},revision:'abc'}}},data:{}};
 return {program:{id:'sample',adapter_digest:'new',package:pkg},list:{integrations:[{id:'sample',next_due:1000}]},history:{runs:[{id:2,trigger:'manual',status:'succeeded',started_at:10,finished_at:11,elapsed_ms:1000,adapter_digest:'new',steps:3,metrics:{retries:1},records:[record],evidence:[{operation:'read',parameters:{path:'ci.yml'},status:'observed'}]}],external_outcomes:[]},records:{as_of:11,records:[{record,expired:false},{record:{...record,kind:'fact',key:'fact:one',model:{entity:{kind:'fact',predicate:'status'}},expires_at:1},expired:true}]},revisions:{accepted:null,candidate:'capture',revisions:[{id:'capture',run_id:1,created_at:1,records:1}],diff:{changes:[]}},revision_details:{capture:{revision:{run_id:1,created_at:1,package:{...pkg,source:'function step() { return "old"; }',input:{repository:'org/old'}}}}}};
}
async function snapshot(data){
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'}),d=dom.window.document;
 const node=d.createElement('script');node.id='host-snapshot';node.type='application/json';node.textContent=JSON.stringify(data);d.body.append(node);
 dom.window.eval(d.querySelector('#monitor-app').textContent);await tick();await tick();return dom;
}
test('navigation, record filters, search, and capture evidence work together',async()=>{
 const dom=await snapshot(fixture());try{const d=dom.window.document;
 d.querySelector('.nav [data-view=inventory]').click();assert.equal(d.querySelector('#view-inventory').hidden,false);assert.equal(d.querySelector('#view-overview').hidden,true);
 d.querySelector('#record-filters [data-kind=fact]').click();assert.match(d.querySelector('#record-count').textContent,/1 of 2/);assert.match(d.querySelector('#records').textContent,/EXPIRED/);
 const search=d.querySelector('#record-search');search.value='missing';search.dispatchEvent(new dom.window.Event('input'));assert.match(d.querySelector('#records').textContent,/No records match/);
 search.value='';search.dispatchEvent(new dom.window.Event('input'));d.querySelector('#records [data-run]').click();assert.equal(d.querySelector('#view-activity').hidden,false);assert.match(d.querySelector('#run-detail').textContent,/ci.yml/);assert.match(d.querySelector('#run-detail').textContent,/3 steps · 1 retries/);
 }finally{dom.window.close();}
});
test('comparison separates source and input edits and escapes provider text',async()=>{
 const dom=await snapshot(fixture());try{const d=dom.window.document;
 d.querySelector('.nav [data-view=changes]').click();assert.match(d.querySelector('#program-changes').textContent,/Source edits · \+1 \/ −1 lines/);assert.match(d.querySelector('#program-changes').textContent,/Program input/);assert.match(d.querySelector('#program-changes').textContent,/org\/old/);assert.match(d.querySelector('#program-changes').textContent,/org\/repo/);assert.equal(d.querySelector('#program-changes img'),null);assert.match(d.querySelector('#review').textContent,/No accepted baseline/);assert.equal(d.querySelector('#review [data-action=accept]').disabled,true);
 }finally{dom.window.close();}
});
test('missing snapshot manifests show a clear comparison limit',async()=>{
 const data=fixture();delete data.revision_details;const dom=await snapshot(data);try{assert.match(dom.window.document.querySelector('#program-changes').textContent,/snapshot has catalog summaries only/);}finally{dom.window.close();}
});
test('live startup upgrades a v1 session before requesting v2 operations',async()=>{
 const data=fixture(),calls=[];let migrated=false;
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'}),d=dom.window.document;
 dom.window.fetch=async(path,options)=>{
  calls.push(path);
  if(path==='/v1/host/session'){
   assert.equal(options.method,'POST');assert.equal(options.headers['X-IPE-Monitor'],'1');
   migrated=true;return {ok:true,json:async()=>({connected:true})};
  }
  if(path.startsWith('/v2/')){assert.equal(migrated,true);return {ok:true,json:async()=>({operations:[]})};}
  const body=path.includes('/revisions/')?data.revision_details.capture:path.endsWith('/revisions')?data.revisions:path.endsWith('/explanations')?null:path.endsWith('/records')?data.records:path.endsWith('/runs')?data.history:path==='/v1/integrations'?data.list:data.program;
  return {ok:true,json:async()=>body};
 };
 try{dom.window.eval(d.querySelector('#monitor-app').textContent);await tick();await tick();
  assert.equal(calls[0],'/v1/host/session');assert.ok(calls.some(p=>p.startsWith('/v2/')));
  assert.equal(d.querySelector('#login').hidden,true);assert.match(d.querySelector('#status').textContent,/Live/);
 }finally{dom.window.close();}
});
test('live runs disable overlap, expose pending reads, and recover after errors',async()=>{
 const data=fixture();data.history.runs[0].status='running';data.history.runs[0].evidence.push({operation:'next_read',status:'pending'});
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'}),d=dom.window.document;let posts=0;
 dom.window.fetch=async(path,options)=>{
  if(path==='/v1/host/session')return {ok:true,json:async()=>({connected:true})};
  if(options.method==='POST'){posts++;return {ok:false,status:409,json:async()=>({code:'integration_busy'})};}
  const body=path.includes('/revisions/')?data.revision_details.capture:path.endsWith('/revisions')?data.revisions:path.endsWith('/explanations')?null:path.endsWith('/records')?data.records:path.endsWith('/runs')?data.history:path==='/v1/integrations'?data.list:data.program;
  return {ok:true,json:async()=>body};
 };
 try{dom.window.eval(d.querySelector('#monitor-app').textContent);await tick();await tick();
 assert.equal(d.querySelector('#run').disabled,true);assert.match(d.querySelector('#run-detail').textContent,/Final metrics pending/);assert.match(d.querySelector('#run-detail').textContent,/next_read/);
 data.history.runs[0].status='succeeded';d.querySelector('#refresh').click();await tick();assert.equal(d.querySelector('#run').disabled,false);
 d.querySelector('#run').click();d.querySelector('#run').click();await tick();assert.equal(posts,1);assert.match(d.querySelector('#notice').textContent,/already running/);assert.equal(d.querySelector('#run').disabled,false);
 }finally{dom.window.close();}
});
test('slow refreshes do not overlap or replace a newly selected installation',async()=>{
 const data=fixture();data.list.integrations.push({id:'second'});
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'}),d=dom.window.document;
 let firstRequests=0,release;
 dom.window.fetch=async path=>{
  if(path==='/v1/integrations/sample'){firstRequests++;await new Promise(resolve=>{release=resolve});}
  const body=path.includes('/revisions/')?data.revision_details.capture:path.endsWith('/revisions')?data.revisions:path.endsWith('/explanations')?null:path.endsWith('/records')?data.records:path.endsWith('/runs')?data.history:path==='/v1/integrations'?data.list:{...data.program,id:path.endsWith('/second')?'second':'sample'};
  return {ok:true,json:async()=>body};
 };
 try{dom.window.eval(d.querySelector('#monitor-app').textContent);await tick();
 d.querySelector('#refresh').click();assert.equal(firstRequests,1,'same-installation polling stays bounded');
 d.querySelector('#integration').value='second';d.querySelector('#integration').dispatchEvent(new dom.window.Event('change'));await tick();await tick();assert.match(d.querySelector('#overview').textContent,/second/);
 release();await tick();assert.match(d.querySelector('#overview').textContent,/second/,'late response cannot replace the selected integration');
 }finally{dom.window.close();}
});

test('progress stays distinct from native evidence and escapes report text',async()=>{
 const data=fixture();data.history.runs[0].progress=[{step:1,reported:{phase:'dispatch',task:'<img src=x>',queued:2,completed:0,gaps:1}}];
 const dom=await snapshot(data);try{
  const d=dom.window.document;d.querySelector('.nav [data-view=activity]').click();
  const detail=d.querySelector('#run-detail');assert.match(detail.textContent,/Program-reported work/);
  assert.match(detail.textContent,/Latest 1 of 1 reports/);assert.match(detail.textContent,/Native reads/);
  assert.match(detail.textContent,/<img src=x>/);assert.equal(detail.querySelector('img'),null);
 }finally{dom.window.close();}
});

test('context assessment submits selected context and renders cross-source evidence without authorizing',async()=>{
 const data=fixture(),example=JSON.parse(fs.readFileSync(__dirname+'/../../testing/integration/hosted/cross-source-assessment.json','utf8'));
 const model=example.models.ci[0],entity=model.entity;
 const logical='control:'+JSON.stringify([entity.control.namespace,entity.control.resource,entity.control.operation,entity.control.local_id]);
 data.explanations={operations:[{...entity,logical_key:logical,scope:model.scope,status:'unknown',reason:'Not authorization',requirements:[],evidence:{},outcomes:[]}],evidence_rules:example.evidence_rules};
 const pin='sha256:'+'a'.repeat(64);
 data.operations={schema:'ipe.integration.operations.v2',operations:[{...entity,operation:entity.control,title:'Canonical release operation',record_revision:pin,declared_enforcer:entity.enforcer,requirements:[]}]};
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'}),d=dom.window.document;let submitted,submittedPath;
 dom.window.fetch=async(path,options)=>{
  if(path==='/v1/host/session')return {ok:true,json:async()=>({connected:true})};
  if(path.endsWith('/assessments')){submittedPath=path;submitted=JSON.parse(options.body);return {ok:true,json:async()=>({as_of:100,context:submitted.context,reason:'Not an authorization decision',requirements:[{id:'approval',description:'Ticket approval',rule:'ticket_rule',fulfillment:{status:'unknown'},evidence_check:{status:'matched',reason:'exact_evidence_match',associations:[],evidence:[{predicate:'approved',value:'<img src=x>',provenance:{integration:'tickets',observed_at:90,expires_at:110}}]}}]})};}
  const body=path.includes('/revisions/')?data.revision_details.capture:path.endsWith('/revisions')?data.revisions:path.endsWith('/operations')?data.operations:path.endsWith('/explanations')?data.explanations:path.endsWith('/records')?data.records:path.endsWith('/runs')?data.history:path==='/v1/integrations'?data.list:data.program;
  return {ok:true,json:async()=>body};
 };
 try{
  dom.window.eval(d.querySelector('#monitor-app').textContent);await tick();await tick();
  const select=d.querySelector('#operation-filter');
  d.querySelector('.nav [data-view=operations]').click();assert.equal(d.querySelector('#view-operations').hidden,false);assert.equal(d.querySelector('#view-inventory #explanations'),null);assert.match(select.selectedOptions[0].textContent,/Canonical release operation/);
  for(const input of d.querySelectorAll('[data-context]'))input.value=example.request.context[input.dataset.context];
  d.querySelector('#assess-context').click();await tick();
  assert.equal(submittedPath,'/v2/integrations/sample/assessments');assert.deepEqual(submitted,{operation:entity.control,source_revision:entity.source.revision,record_revision:pin,context:example.request.context});assert.match(d.querySelector('#assessment-result').textContent,/Requirement fulfillment: unknown/);assert.match(d.querySelector('#assessment-result').textContent,/Evidence check: matched/);assert.match(d.querySelector('#assessment-result').textContent,/tickets reports approved/);
  assert.match(d.querySelector('#assessment-result').textContent,/Not an authorization decision/);assert.equal(d.querySelector('#assessment-result img'),null);
  data.operations.operations[0].record_revision='sha256:'+'b'.repeat(64);d.querySelector('#refresh').click();await tick();await tick();assert.equal(d.querySelector('#assessment-result').textContent,'','changed exact binding clears old evidence check');
 }finally{dom.window.close();}
});

test('legacy snapshots expose operation descriptions but cannot run unpinned evidence checks',async()=>{
 const data=fixture();data.explanations={operations:[{control:{namespace:'fixture'},title:'Legacy operation',logical_key:'old',source:{definition:{resource:'repo',path:'ci'},revision:'source'},requirements:[{id:'r',description:'Approval',requirement:{kind:'opaque'},assessment:{status:'satisfied',reason:'old matching evidence'}}]}]};
 const dom=await snapshot(data);try{const d=dom.window.document;assert.match(d.querySelector('#explanations').textContent,/Legacy operation/);assert.match(d.querySelector('#explanations').textContent,/Requirement fulfillment: Unknown/);assert.doesNotMatch(d.querySelector('#explanations').textContent,/fulfillment: satisfied/);assert.match(d.querySelector('#assessment-inputs').textContent,/no exact mapped operation revision/);assert.equal(d.querySelector('#assess-context'),null);}finally{dom.window.close();}
});

test('two primary journeys group details and cross-links select the correct journey',async()=>{
 const dom=await snapshot(fixture());try{const d=dom.window.document;
 assert.equal(d.querySelectorAll('[data-journey]').length,2);
 assert.equal(d.querySelector('[data-group=coverage]').hidden,true);
 assert.equal(d.querySelector('#view-overview').hidden,false);
 d.querySelector('[data-journey=coverage]').click();
 assert.equal(d.querySelector('#view-governance').hidden,false);
 assert.equal(d.querySelector('[data-group=runtime]').hidden,true);
 assert.equal(d.querySelector('[data-group=coverage]').hidden,false);
 d.querySelector('.nav [data-view=inventory]').click();
 d.querySelector('#records [data-run]').click();
 assert.equal(d.querySelector('#view-activity').hidden,false);
 assert.equal(d.querySelector('[data-journey=runtime]').getAttribute('aria-current'),'page');
 assert.equal(d.querySelector('[data-group=coverage]').hidden,true);
 assert.equal(d.querySelector('.nav [data-view=activity]').getAttribute('aria-current'),'page');
 }finally{dom.window.close();}
});

test('failed connection shows one connection instruction and keeps token entry optional',async()=>{
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'}),d=dom.window.document;
 dom.window.fetch=async()=>({ok:false,status:401});
 try{dom.window.eval(d.querySelector('#monitor-app').textContent);await tick();await tick();
 assert.equal(d.querySelector('#login').hidden,false);
 assert.equal(d.querySelector('#login details').open,false);
 assert.equal(d.querySelector('#notice').textContent,'');
 assert.equal(dom.window.getComputedStyle(d.querySelector('.workspace')).display,'none');
 assert.equal(d.querySelector('#run').disabled,true);
 }finally{dom.window.close();}
});
