const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {JSDOM}=require('../../docs/node_modules/jsdom');
test('snapshot separates collector time from external duration and escapes provider text', async()=>{
 const html=fs.readFileSync(__dirname+'/index.html','utf8');
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'});
 try {
  const d=dom.window.document, data={program:{id:'sample',adapter_digest:'sha256:code',package:{source:'// <img src=x>',input:{repository:'org/repo'},enabled:true,interval_seconds:300,fact_ttl_seconds:900,operations:['read']}},list:{integrations:[{id:'sample'}]},history:{runs:[{id:1,trigger:'manual',status:'succeeded',elapsed_ms:100,metrics:{requests:1,retries:0}}],external_outcomes:[{key:'attempt/1',data:{name:'<img src=x onerror=alert(1)>',duration_ms:26000,conclusion:'success'}}]},records:{records:[{expired:true,record:{kind:'fact',key:'attempt/1',observed_at:1,run_id:1,data:{status:'success'},source_evidence:[{source_digest:'sha256:test'}]}}]}};
  const node=d.createElement('script');node.id='host-snapshot';node.type='application/json';node.textContent=JSON.stringify(data);d.body.append(node);
  dom.window.eval(d.querySelector('script').textContent);
  await new Promise(resolve=>setTimeout(resolve,0));
  assert.match(d.querySelector('#runs').textContent,/100 ms/);
  assert.match(d.querySelector('#outcomes').textContent,/26000 ms/);
  assert.match(d.querySelector('#outcomes').textContent,/Unknown/);
  assert.equal(d.querySelector('#outcomes img'),null);
  assert.match(d.querySelector('#records').textContent,/EXPIRED/);
  assert.match(d.querySelector('#records').textContent,/sha256:test/);
  assert.equal(d.querySelector('#run').disabled,true);
  assert.match(d.querySelector('#program').textContent,/Every 300 seconds/);
  assert.match(d.querySelector('#program').textContent,/org\/repo/);
  assert.equal(d.querySelector('#program img'),null);
  assert.equal(d.querySelector('#login').hidden,true);
  const details=d.querySelector('#program details');details.open=true;details.querySelector('summary').focus();
  d.querySelector('#refresh').click();await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(d.querySelector('#program details').open,true,'refresh preserves open source');
  const updated=JSON.parse(node.textContent);updated.program.package.source+='\n// new source';node.textContent=JSON.stringify(updated);
  d.querySelector('#refresh').click();await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(d.querySelector('#program details').open,true,'changed source also preserves disclosure state');
  assert.equal(d.activeElement,d.querySelector('#program summary'),'keyboard focus stays on the disclosure');
  d.querySelector('#records details').open=true;
  updated.records.records[0].expired=false;node.textContent=JSON.stringify(updated);
  d.querySelector('#refresh').click();await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(d.querySelector('#records details').open,true,'freshness label changes preserve the same record panel');
 } finally {dom.window.close();}
});
test('operation view shows unknowns and granted observations, and snapshots disable revision actions', async()=>{
 const html=fs.readFileSync(__dirname+'/index.html','utf8');
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'});
 try {
  const d=dom.window.document;
  const data={list:{integrations:[{id:'sample'}]},program:{},history:{runs:[],external_outcomes:[]},records:{records:[]},
   explanations:{operations:[{logical_key:'control',title:'Publish <script>',status:'unknown',reason:'Actor is unknown',applicability:{expression:'on main'},actor_context:'unknown',enforcer:'provider',requirements:[{id:'dependency',description:'Build must succeed',assessment:{status:'unknown',reason:'No fresh fact',evidence:[]},requirement:{kind:'dependency'},next_observation:{operation:'list_jobs',reason:'Observe attempt'}}],source:{definition:{resource:'repo',path:'ci.yml'},revision:'abc'},evidence:{observed_at:1,run_id:1},outcomes:[],dependencies:[]}],coverage:{run_id:1,status:'succeeded',reads:[{status:'unavailable',code:'provider_access_denied'}],complete_declaration_scopes:[]}},
   revisions:{accepted:'old',candidate:'new',revisions:[{id:'old',created_at:1,run_id:1,accepted_at:1}],diff:{changes:[{logical_key:'control',category:'requirement_changed'}]}}};
  const node=d.createElement('script');node.id='host-snapshot';node.type='application/json';node.textContent=JSON.stringify(data);d.body.append(node);
  dom.window.eval(d.querySelector('script').textContent);await new Promise(r=>setTimeout(r,0));
  assert.match(d.querySelector('#explanations').textContent,/Actor is unknown/);
  assert.match(d.querySelector('#explanations').textContent,/list_jobs/);
  assert.equal(d.querySelector('#explanations script'),null);
  assert.match(d.querySelector('#coverage').textContent,/1 unavailable reads/);
  assert.match(d.querySelector('#review').textContent,/requirement changed/);
  assert.equal(d.querySelectorAll('#review button:disabled').length,2);
  assert.ok(d.querySelector('main'));
 }finally{dom.window.close();}
});
