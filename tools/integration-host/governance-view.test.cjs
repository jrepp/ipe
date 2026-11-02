const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {JSDOM}=require('../../docs/node_modules/jsdom');
const script=fs.readFileSync(__dirname+'/governance-view.js','utf8');
function fixture(){
 const source={definition:{namespace:'fixture',resource:'org/repo',path:'release'},revision:'rev1',pointer:''};
 const id=local_id=>({namespace:'fixture',authority:'org',local_id});
 const ref=(type,name,resolution='resolved',revision='rev1')=>({kind:type,id:id(name),source_revision:revision,resolution});
 const gov=(type,name,fields)=>({kind:'governance',key:name,revision_id:'mapped:'+name,receipts:['receipt:1'],observed_at:1,model:{schema_version:2,entity:{kind:'governance',record:{type,id:id(name),source,title:name,...fields}}}});
 const control={namespace:'fixture',resource:'org/repo',operation:'release',local_id:'ship'},operationKey='control:'+JSON.stringify(Object.values(control));
 const execution={namespace:'fixture',resource:'org/repo',run:'42',attempt:1,job:null,step:null};
 return {id:1,status:'succeeded',records:[
  gov('principal','release-team',{principal_kind:'role'}),gov('principal','reviewer',{principal_kind:'human'}),
  gov('policy','Release approval',{authority:ref('principal','release-team'),subjects:[operationKey],requirements:[{id:'approval',description:'Get approval',specification:'A reviewer must approve release.'}]}),
  gov('control','Approval gate',{governing_rules:[ref('policy','Release approval')],mechanism:'The provider blocks release until approval.'}),
  gov('handoff','Request review',{policy:ref('policy','Release approval'),from:ref('principal','release-team'),to:ref('principal','reviewer'),trigger:'Release requested',required_action:'Review the release',completion:'Record approval',continuation:'Resume release'}),
  gov('role_assignment','reviewer-role',{principal:ref('principal','reviewer'),role:ref('principal','release-team'),relationship:'approval_authority',valid_from:null,valid_until:1}),
  gov('contact_method','reviewer-contact',{owner:ref('principal','reviewer'),channel:'email',endpoint:'<img src=x onerror=alert(1)>',availability:'available',purpose:'Request review',valid_until:1}),
  gov('contact_method','missing-contact',{owner:ref('principal','absent','candidate',null),channel:'team_channel',endpoint:null,availability:'unknown',purpose:'Escalation',valid_until:null}),
  {kind:'constraint',model:{entity:{kind:'constraint',control,source,title:'Release operation'}}},
  {kind:'outcome',model:{entity:{kind:'outcome',execution,definition:source,title:'Release execution',declaration_candidates:[control],status:'completed',conclusion:'success'}}},
  {kind:'fact',expires_at:1,model:{entity:{kind:'fact',subject:'release',predicate:'approved',value:'<script>bad()</script>',execution}}}
 ],evidence:[{step:1,operation:'read_governance',status:'observed',receipt:'receipt:1'}]};
}
function setup(){const dom=new JSDOM('<div id="view"></div>',{runScripts:'outside-only'});dom.window.eval(script);return {dom,d:dom.window.document,render:(run,installation)=>dom.window.IpeGovernanceView.render(dom.window.document.querySelector('#view'),run,installation)};}
test('plain explanation keeps rule scope, candidate execution and exact observations distinct',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture();render(run);
  let story=d.querySelector('.ipe-coverage-story');
  assert.match(story.textContent,/A reviewer must approve release/);
  assert.match(story.textContent,/rule lists these operation identities/);
  assert.match(story.textContent,/Provider result: success/);
  assert.match(story.textContent,/Possible run of: Release operation/);
  assert.match(story.textContent,/connection is not confirmed/);
  assert.match(story.textContent,/same execution/);assert.match(story.textContent,/Expired/);
  assert.match(story.textContent,/We cannot tell from this capture/);
  assert.equal(story.querySelector('script,img'),null);
  const outcome=run.records.find(r=>r.model.entity.kind==='outcome');
  outcome.model.entity.definition={...outcome.model.entity.definition,revision:'unrelated'};render(run);
  story=d.querySelector('.ipe-coverage-story');
  assert.match(story.textContent,/No related execution was captured/);
  assert.doesNotMatch(story.textContent,/Provider result: success/);
  run.records.find(r=>r.model.entity.record?.type==='policy').model.entity.record.subjects=[];render(run);
  assert.match(d.querySelector('.ipe-coverage-story').textContent,/No operation is linked/);
 }finally{dom.window.close();}
});
test('an empty governance capture exposes denied API reads instead of implying policy absence',()=>{
 const {dom,d,render}=setup();try{
  render({id:1,status:'succeeded',records:[],summary:{gaps:[{code:'provider_access_denied',path:'list_environments'}]}});
  const gaps=d.querySelector('.ipe-governance-collection-gaps');
  assert.equal(gaps.open,true);assert.match(gaps.textContent,/provider access denied.*list_environments/);
  assert.match(d.querySelector('#view').textContent,/No governance descriptions were retained/);
 }finally{dom.window.close();}
});
test('governance inspection separates rules, handoffs, contacts, collection, and candidate execution',()=>{
 const {dom,d,render}=setup();try{render(fixture());const body=d.querySelector('.ipe-governance-body');assert.match(body.textContent,/Get approval/);assert.match(body.textContent,/Collected at1970-01-01T00:00:01.000Z/);assert.match(d.querySelector('.ipe-governance-overlay').textContent,/Policy applicability at execution time: Unknown/);assert.match(d.querySelector('.ipe-governance-overlay').textContent,/Collection time does not establish when this rule took effect/);assert.match(body.textContent,/Review the release/);assert.match(body.textContent,/Resume release/);assert.match(body.textContent,/Source identity/);assert.match(body.textContent,/fixture \/ org \/ reviewer/);assert.match(body.textContent,/Expired · historical description only/);assert.match(body.textContent,/1 successful native reads \/ 1 receipt references/);assert.match(d.querySelector('.ipe-governance-overlay').textContent,/Candidate execution link · success/);assert.match(d.querySelector('.ipe-governance-overlay').textContent,/1 facts have this exact execution identity/);assert.match(body.textContent,/Requirement fulfillment: Unknown/);assert.equal(body.querySelector('img,script,a'),null);assert.match(body.textContent,/<img src=x/);assert.equal(d.querySelector('.ipe-governance-raw').open,false);assert.match(d.querySelector('.ipe-governance-gaps').textContent,/absent.*Target not captured/);}finally{dom.window.close();}
});
test('same receipt or title never joins governance to an unrelated execution',()=>{
 const {dom,d,render}=setup();try{const run=fixture();run.records.find(r=>r.model.entity.kind==='outcome').model.entity.definition={...run.records[0].model.entity.record.source,revision:'another-revision'};render(run);assert.match(d.querySelector('.ipe-governance-overlay').textContent,/No captured governance-to-execution link/);assert.doesNotMatch(d.querySelector('.ipe-governance-overlay').textContent,/Candidate execution link/);
 run.records.find(r=>r.model.entity.record?.type==='policy').model.entity.record.subjects=[];run.records.find(r=>r.model.entity.kind==='outcome').model.entity.definition=run.records[0].model.entity.record.source;render(run);assert.match(d.querySelector('.ipe-governance-overlay').textContent,/No captured governance-to-execution link/);}finally{dom.window.close();}
});
test('reference revisions and ambiguity remain gaps and unchanged refresh preserves focus',()=>{
 const {dom,d,render}=setup();try{const run=fixture();const principal=run.records[1];run.records.push({...principal,revision_id:'duplicate'});render(run);assert.match(d.querySelector('.ipe-governance-gaps').textContent,/Ambiguous reference/);const select=d.querySelector('.ipe-governance-select');select.focus();render(run);assert.equal(d.querySelector('.ipe-governance-select'),select);assert.equal(d.activeElement,select);
 render({id:2,status:'failed',records:[],trace:{emissions:[{kind:'governance'}]}});assert.match(d.querySelector('#view').textContent,/No governance descriptions/);assert.match(d.querySelector('#view').textContent,/Policy applicability at execution time: Unknown/);assert.equal(d.querySelector('.ipe-governance-select'),null);assert.doesNotMatch(d.querySelector('#view').textContent,/Release approval/);}finally{dom.window.close();}
});

test('plain explanation precedes collapsed record connections',()=>{
 const {dom,d,render}=setup();try{const run=fixture();run.records.find(r=>r.model.entity.record?.type==='control').model.entity.record.governing_rules[0].source_revision='stale-source';run.evidence=[];render(run);const body=d.querySelector('.ipe-governance-body');assert.equal(body.children[0].className,'ipe-coverage-story');assert.equal(body.children[1].className,'ipe-coverage-technical');assert.equal(body.children[1].open,false);assert.ok(body.children[1].contains(d.querySelector('.ipe-governance-mapping')));assert.equal(d.querySelector('.ipe-governance-mechanisms').open,false);assert.equal(d.querySelector('.ipe-governance-people').open,false);assert.doesNotMatch(d.querySelector('.ipe-governance-mechanisms').textContent,/Approval gate/);assert.match(d.querySelector('.ipe-governance-gaps').textContent,/Approval gate.*Target not captured/);assert.match(d.querySelector('.ipe-governance-handoff-summary').textContent,/Release requested → reviewer.*→ Resume release/);assert.match(d.querySelector('.ipe-governance-handoff-summary').textContent,/email: <img src=x/);assert.match(d.querySelector('.ipe-governance-support').textContent,/Extraction support gap/);assert.match(d.querySelector('.ipe-governance-record').textContent,/ClaimUnknown/);}finally{dom.window.close();}
});
test('shared governance fixture exposes all seven source descriptions without inventing a missing contact',()=>{
 const fixture=JSON.parse(fs.readFileSync(__dirname+'/../../testing/integration/hosted/governance-handoff.json','utf8'));
 const {dom,d,render}=setup();try{render({id:14,status:'succeeded',records:fixture.models.map((model,i)=>({model,kind:model.entity.kind,key:String(i),revision_id:'mapped:'+i,receipts:[]})),evidence:[]});const kinds=new Set(fixture.models.filter(model=>model.entity.kind==='governance').map(model=>model.entity.record.type));assert.equal(kinds.size,7);assert.match(d.querySelector('.ipe-governance-overlay').textContent,/Candidate execution link/);assert.match(d.querySelector('.ipe-governance-gaps').textContent,/Target not captured/);assert.match(d.querySelector('.ipe-governance-inventory').textContent,/Not captured/);assert.equal(d.querySelector('a'),null);}finally{dom.window.close();}
});

test('mapping explains source configuration, candidate links, fact emissions, and current collector settings',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture(),fact=run.records.find(r=>r.model.entity.kind==='fact');
  Object.assign(fact,{kind:'fact',key:'fact:approved',revision_id:'fact-rev',receipts:['receipt:1']});
  run.trace={emissions:[{kind:'fact',key:'fact:approved',revision_id:'fact-rev',step:9},{kind:'fact',key:'fact:approved',revision_id:'wrong-rev',step:99}]};
  const installation={adapter_digest:'collector-current',package:{operations:['read_governance'],output_kinds:['governance','fact'],enabled:true,interval_seconds:300,fact_ttl_seconds:600}};
  render(run,installation);
  const mapping=d.querySelector('.ipe-governance-mapping');
  assert.match(mapping.textContent,/Get approval.*A reviewer must approve release/);
  assert.match(mapping.textContent,/fixture|org\/repo/);
  assert.match(mapping.textContent,/Exact retained reference/);
  assert.match(mapping.textContent,/Candidate execution link/);
  assert.match(mapping.textContent,/Record emitted at collector step 9/);
  assert.doesNotMatch(mapping.textContent,/collector step 99/);
  assert.match(mapping.textContent,/Expired evidence/);
  assert.match(mapping.textContent,/not automatically an IPE runtime fact/);
  assert.equal(mapping.querySelector('script,img'),null);
  const settings=d.querySelector('.ipe-governance-settings');
  assert.match(settings.textContent,/installed now/);assert.match(settings.textContent,/300 seconds/);
  assert.match(settings.textContent,/do not define external policy/);
  installation.package.interval_seconds=900;render(run,installation);
  assert.match(d.querySelector('.ipe-governance-settings').textContent,/900 seconds/);
 }finally{dom.window.close();}
});
test('mapping exposes unmatched scope, missing evidence, and candidate controls without promoting them',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture();
  run.records.find(r=>r.model.entity.record?.type==='policy').model.entity.record.subjects=['provider-scope:not-an-operation'];
  run.records.find(r=>r.model.entity.record?.type==='control').model.entity.record.governing_rules[0].resolution='candidate';
  render(run);
  const mapping=d.querySelector('.ipe-governance-mapping');
  assert.match(mapping.textContent,/Candidate reference/);assert.match(mapping.textContent,/provider-scope:not-an-operation/);
  assert.match(mapping.textContent,/Mapping gap/);assert.match(mapping.textContent,/Evidence gap/);
  assert.doesNotMatch(mapping.querySelector('[data-concept=Control]').textContent,/Exact retained reference/);
  assert.doesNotMatch(mapping.textContent,/Candidate execution link|Integration fact/);
  assert.match(d.querySelector('.ipe-governance-guide').textContent,/contract: 0/);
  assert.match(d.querySelector('.ipe-governance-settings').textContent,/Installation settings are not included/);
 }finally{dom.window.close();}
});

test('requirement focus preserves source fields and does not infer fulfillment from a matching fact name',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture(),policy=run.records.find(r=>r.model.entity.record?.type==='policy').model.entity.record;
  policy.requirements.push({id:'branch',description:'Deployment branch policy',specification:JSON.stringify({custom_branch_policies:true,protected_branches:false,unknown_setting:null,note:'<img src=x onerror=alert(1)>'})});
  render(run);
  let choice=d.querySelector('.ipe-governance-requirement-select');
  assert.equal(choice.options.length,2);
  assert.match(d.querySelector('.ipe-governance-binding').textContent,/Requirement-to-fact binding: Not retained/);
  choice.value='1';choice.dispatchEvent(new dom.window.Event('change'));
  const spec=d.querySelector('.ipe-governance-mapping .ipe-governance-specification');
  assert.match(spec.textContent,/custom_branch_policiestrue/);assert.match(spec.textContent,/protected_branchesfalse/);
  assert.match(spec.textContent,/unknown_settingnull \(source value\)/);
  assert.match(spec.textContent,/<img src=x/);assert.equal(spec.querySelector('img'),null);
  assert.equal(d.activeElement,d.querySelector('.ipe-governance-requirement-select'));
  assert.match(d.querySelector('.ipe-governance-binding').textContent,/Selected requirement: Deployment branch policy/);
  assert.doesNotMatch(spec.textContent,/A reviewer must approve/);
  run.evidence.push({operation:'another_read',status:'unavailable'});render(run);
  assert.equal(d.querySelector('.ipe-governance-requirement-select').value,'1');
  render({...run,id:2});assert.equal(d.querySelector('.ipe-governance-requirement-select').value,'0');
 }finally{dom.window.close();}
});
test('opaque and truncated requirement specifications stay literal and structured previews are bounded',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture(),requirement=run.records.find(r=>r.model.entity.record?.type==='policy').model.entity.record.requirements[0];
  requirement.specification='{"required_reviewers": [<script>bad()</script>';
  render(run);let spec=d.querySelector('.ipe-governance-specification');
  assert.match(spec.textContent,/No structured interpretation/);assert.match(spec.textContent,/<script>bad/);assert.equal(spec.querySelector('script'),null);
  requirement.specification=JSON.stringify(Object.fromEntries(Array.from({length:20},(_,i)=>['field_'+i,i])));
  render(run);spec=d.querySelector('.ipe-governance-specification');
  assert.equal(spec.querySelectorAll('dt').length,16);assert.match(spec.textContent,/Showing 16 of 20 fields/);
  assert.equal(spec.querySelector('pre').textContent,requirement.specification);
 }finally{dom.window.close();}
});
test('saved runtime configuration does not claim to describe the current installation',()=>{
 const {dom,d,render}=setup();try{
  render(fixture(),{snapshot:true,package:{operations:[],output_kinds:[],enabled:false,interval_seconds:300,fact_ttl_seconds:600}});
  const settings=d.querySelector('.ipe-governance-settings');
  assert.match(settings.textContent,/retained snapshot/);assert.match(settings.textContent,/do not establish the current installation/);
  assert.doesNotMatch(settings.textContent,/installed now|These are current installation settings/);
 }finally{dom.window.close();}
});

test('concept map opens one detail group and preserves selection across refresh',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture();render(run);
  const visible=()=>[...d.querySelectorAll('.ipe-ontology-detail')].filter(node=>!node.hidden);
  assert.deepEqual(visible().map(node=>node.dataset.concept),['Requirement']);
  const control=d.querySelector('button[data-concept=Control]');control.focus();control.click();
  assert.deepEqual(visible().map(node=>node.dataset.concept),['Control']);
  assert.match(visible()[0].textContent,/provider blocks release/);
  assert.equal(control.getAttribute('aria-pressed'),'true');
  render(run);assert.equal(d.activeElement,control);
  run.evidence.push({operation:'read_more',status:'unavailable'});render(run);
  assert.equal(d.activeElement.dataset.concept,'Control');
  assert.deepEqual(visible().map(node=>node.dataset.concept),['Control']);
  render({...run,id:2});assert.deepEqual(visible().map(node=>node.dataset.concept),['Requirement']);
  assert.equal(d.querySelector('.ipe-governance-guide').open,false);
  assert.equal(d.querySelector('.ipe-governance-overlay').open,false);
 }finally{dom.window.close();}
});

test('gap filter retains uncertain concepts and does not promote candidate or expired mappings',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture();run.records.find(r=>r.model.entity.record?.type==='control').model.entity.record.governing_rules[0].resolution='candidate';
  run.records.find(r=>r.model.entity.record?.type==='handoff').model.entity.record.to.source_revision='missing';
  render(run);d.querySelector('.ipe-ontology-filter').click();
  const node=concept=>[...d.querySelectorAll('.ipe-ontology-node')].find(n=>n.dataset.concept===concept);
  assert.equal(node('policy').hidden,true);assert.equal(node('Operation').hidden,true);
  for(const concept of ['Requirement','Control','Handoff','Principal','External outcome','Integration fact','Policy applicability'])assert.equal(node(concept).hidden,false,concept);
  assert.match(node('Control').textContent,/Candidate reference/);
  assert.match(node('Handoff').textContent,/Target not captured/);
  assert.match(node('Integration fact').textContent,/Expired evidence/);
  node('Handoff').click();assert.match(d.querySelector('.ipe-ontology-detail[data-concept=Handoff]').textContent,/missing|Target not captured/);
  d.querySelector('.ipe-ontology-filter').click();assert.equal(node('policy').hidden,false);
 }finally{dom.window.close();}
});

test('a changed rule clears the selected concept and gap filter',()=>{
 const {dom,d,render}=setup();try{
  const run=fixture(),policy=structuredClone(run.records.find(r=>r.model.entity.record?.type==='policy'));
  policy.model.entity.record.id.local_id='second';policy.model.entity.record.title='Second rule';policy.model.entity.record.requirements=[];run.records.push(policy);
  render(run);d.querySelector('.ipe-ontology-filter').click();
  const select=d.querySelector('.ipe-governance-select');select.selectedIndex=1;select.dispatchEvent(new dom.window.Event('change'));
  assert.equal(d.querySelector('.ipe-ontology-filter').getAttribute('aria-pressed'),'false');
  assert.equal(d.querySelector('.ipe-ontology-node[aria-pressed=true]').dataset.concept,'Requirement');
  assert.match(d.querySelector('.ipe-ontology-detail[data-concept=Requirement]').textContent,/No named requirement/);
  assert.match(d.querySelector('.ipe-ontology-detail[data-concept=Control]').textContent,/No mechanism references/);
 }finally{dom.window.close();}
});
