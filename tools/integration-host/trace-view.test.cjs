const {test} = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
const {JSDOM} = require('../../docs/node_modules/jsdom');
const tick = () => new Promise(resolve => setTimeout(resolve, 0));

function fixture() {
  const untrusted = '</script><img src=x onerror="globalThis.providerInjected=true">';
  const fact = {
    kind:'fact', key:'fact:approval', revision_id:'revision:approval', run_id:1,
    adapter_digest:'program', observed_at:10, expires_at:900, receipts:['receipt:approval'],
    model:{schema_version:1, scope:'fixture', claim:'provider_reported', entity:{
      kind:'fact', subject:'ticket:42', predicate:'approved', value:untrusted, authority:'fixture', execution:null,
    }}, data:{provider:untrusted},
  };
  const definition = {
    kind:'definition', key:'definition:release', revision_id:'revision:release', run_id:1,
    receipts:['receipt:approval'], model:{entity:{kind:'definition', title:'Release definition',
      source:{definition:{namespace:'fixture',resource:'service',path:'release'},revision:'source:1',pointer:''}}}, data:{},
  };
  const success = {
    id:1, trigger:'manual', status:'succeeded', started_at:10, finished_at:11, elapsed_ms:1000,
    adapter_digest:'program', steps:4, metrics:{retries:0}, records:[definition,fact],
    evidence:[{step:1,request_id:'read_approval',operation:'read_ticket',parameters:{ticket:'42'},status:'observed',receipt:'receipt:approval',observed_at:10}],
    trace:{schema_version:1,emissions:[
      {step:2,key:definition.key,kind:definition.kind,revision_id:definition.revision_id,receipts:definition.receipts},
      {step:3,key:fact.key,kind:fact.kind,revision_id:fact.revision_id,receipts:fact.receipts},
    ],omitted:0},
  };
  const control={namespace:'fixture',resource:'service',operation:'release',local_id:'release'};
  const execution={namespace:'fixture',resource:'service',run:'42',attempt:1,job:null,step:null};
  fact.model.entity.execution=execution;
  const operation={...definition,kind:'constraint',key:'operation:release',revision_id:'revision:operation',model:{entity:{kind:'constraint',control,title:'Release operation',source:definition.model.entity.source,actor_context:'unknown',enforcer:'provider',requirements:[{id:'approval',description:'Approval required',requirement:{kind:'opaque'}}]}}};
  const outcome={...definition,kind:'outcome',key:'outcome:release',revision_id:'revision:outcome',model:{entity:{kind:'outcome',title:'Release execution',execution,definition:definition.model.entity.source,declaration_candidates:[control],correlation:'candidate',status:'completed',conclusion:'success'}}};
  success.records.push(operation,outcome);
  success.trace.emissions.push(...[operation,outcome].map(record=>({step:4,key:record.key,kind:record.kind,revision_id:record.revision_id,receipts:record.receipts})));
  const legacy = {
    ...success, id:2, steps:88, trace:undefined,
    records:[{...fact,key:'fact:legacy',model:{...fact.model,entity:{...fact.model.entity,predicate:'legacy approval',execution:null}}}],
    evidence:[{operation:'read_legacy',status:'observed',receipt:'receipt:approval'}],
  };
  const failed = {
    ...success,id:3,status:'failed',error:'script_failed',records:[],
    trace:{schema_version:1,emissions:[{step:2,key:'fact:attempted',kind:'fact',revision_id:'revision:attempted',receipts:['receipt:approval']}],omitted:0},
  };
  return {
    program:{id:'sample',adapter_digest:'program',package:{source:'function step() {}',input:{},operations:['read_ticket'],output_kinds:['definition','fact'],enabled:false,interval_seconds:300,fact_ttl_seconds:900}},
    list:{integrations:[{id:'sample'}]},history:{runs:[success,legacy,failed],external_outcomes:[]},
    records:{as_of:11,records:[{record:fact,expired:false}]},
    revisions:{accepted:null,candidate:null,revisions:[],diff:{changes:[]}},
  };
}

async function snapshot(data) {
  const generated = spawnSync('python3', ['-c',
    "import json,runpy,sys; module=runpy.run_path(sys.argv[1]); sys.stdout.write(module['render_snapshot'](json.load(sys.stdin)))",
    path.join(__dirname,'demo.py')], {input:JSON.stringify(data),encoding:'utf8',maxBuffer:4*1024*1024});
  assert.equal(generated.status,0,generated.stderr);
  const dom = new JSDOM(generated.stdout,{runScripts:'outside-only',url:'file:///tmp/ipe-snapshot.html'});
  assert.equal(dom.window.document.querySelector('script[src]'),null,'snapshot embeds all JavaScript assets');
  assert.equal(dom.window.document.querySelector('link[href^="/assets/"]'),null,'snapshot embeds trace styles');
  assert.deepEqual(JSON.parse(dom.window.document.querySelector('#host-snapshot').textContent),JSON.parse(JSON.stringify(data)), 'provider text survives safe JSON embedding');
  for (const script of dom.window.document.querySelectorAll('script:not([type="application/json"])')) dom.window.eval(script.textContent);
  await tick(); await tick();
  assert.equal(dom.window.d3.version,'7.9.0','tests use the bundled D3 implementation');
  return dom;
}

function selectRun(document,id) {
  document.querySelector(`#runs button[data-run="${id}"]`).click();
}
function factNode(document) {
  return [...document.querySelectorAll('#run-trace .ipe-trace-node')].find(node => node.__data__.lane === 'fact');
}

test('offline trace bundles real D3 and renders native fact timing with escaped provider details', async () => {
  const dom = await snapshot(fixture());
  try {
    const d=dom.window.document;
    selectRun(d,1);
    assert.equal(d.querySelector('#view-activity').hidden,false);
    assert.ok(d.querySelector('#run-trace svg'));
    const fact=factNode(d);
    assert.match(fact.getAttribute('aria-label'),/approved.*Step 3/);
    fact.dispatchEvent(new dom.window.MouseEvent('click',{bubbles:true}));
    const detail=d.querySelector('#run-trace .ipe-trace-details');
    assert.match(detail.textContent,/Step 3/);
    assert.match(detail.textContent,/<img src=x/);
    assert.equal(detail.querySelector('img'),null);
    assert.equal(dom.window.providerInjected,undefined);
    assert.ok(d.querySelector('#run-trace .ipe-trace-edge.is-selected'),'selection highlights the retained execution identity');
    assert.match(d.querySelector('#run-trace .ipe-trace-state').textContent,/do not prove requirement fulfillment/);
  } finally {dom.window.close();}
});

test('unchanged snapshot refresh preserves trace selection, focus, and operation selection', async () => {
  const dom=await snapshot(fixture());
  try {
    const d=dom.window.document;
    selectRun(d,1);
    const operation=d.querySelector('.ipe-domain-operation').value;
    assert.equal(d.querySelectorAll('#run-trace .ipe-trace-node').length,3);
    assert.match(d.querySelector('.ipe-domain-link-status').textContent,/Candidate/);
    assert.match(d.querySelector('.ipe-domain-fulfillment').textContent,/Unknown/);
    const fact=factNode(d);
    fact.focus();
    fact.dispatchEvent(new dom.window.KeyboardEvent('keydown',{key:'Enter',bubbles:true}));
    assert.equal(d.activeElement,fact);
    assert.equal(fact.getAttribute('aria-pressed'),'true');
    d.querySelector('#refresh').click();
    await tick();await tick();
    assert.equal(factNode(d),fact,'unchanged data retains the selected DOM node');
    assert.equal(d.activeElement,fact);
    assert.equal(fact.getAttribute('aria-pressed'),'true');
    assert.equal(d.querySelector('.ipe-domain-operation').value,operation);
  } finally {dom.window.close();}
});

test('run switches clear prior graph selection and distinguish failed emissions from legacy unknown timing', async () => {
  const dom=await snapshot(fixture());
  try {
    const d=dom.window.document;
    selectRun(d,1);
    factNode(d).dispatchEvent(new dom.window.MouseEvent('click',{bubbles:true}));
    selectRun(d,3);
    assert.match(d.querySelector('.ipe-trace-heading').textContent,/Run 3/);
    assert.equal(d.querySelector('.ipe-domain-operation').value,'__unmapped__');
    assert.equal(d.querySelector('.ipe-trace-node[aria-pressed="true"]'),null);
    assert.equal(factNode(d).__data__.status,'discarded');
    assert.match(factNode(d).getAttribute('aria-label'),/fact:attempted.*Step 2/);
    assert.equal([...d.querySelectorAll('.ipe-trace-node')].some(node=>node.__data__.label==='approved'),false,'current inventory cannot leak into a failed run graph');
    assert.match(d.querySelector('.ipe-trace-state').textContent,/not committed records/);
    selectRun(d,2);
    assert.match(d.querySelector('.ipe-trace-state').textContent,/Legacy run/);
    assert.match(factNode(d).getAttribute('aria-label'),/Exact step not recorded/);
    assert.doesNotMatch(factNode(d).getAttribute('aria-label'),/Step 88/);
    assert.equal(factNode(d).__data__.step,undefined);
  } finally {dom.window.close();}
});

test('focused diagram pages facts and keeps collection reads behind disclosure',async()=>{
 const data=fixture(),run=data.history.runs[0],fact=run.records.find(record=>record.kind==='fact');
 for(let i=1;i<14;i++){const record={...fact,key:'fact:'+i,revision_id:'revision:'+i};run.records.push(record);run.trace.emissions.push({step:4,key:record.key,kind:'fact',revision_id:record.revision_id,receipts:record.receipts});}
 const dom=await snapshot(data);try{const d=dom.window.document;selectRun(d,1);assert.equal(d.querySelectorAll('.ipe-trace-node').length,8);assert.match(d.querySelector('.ipe-domain-page').textContent,/Facts 1–6 of 14/);assert.equal(d.querySelector('.ipe-domain-collection').open,false);const first=factNode(d).__data__.id;d.querySelector('.ipe-domain-next').click();assert.match(d.querySelector('.ipe-domain-page').textContent,/Facts 7–12 of 14/);assert.notEqual(factNode(d).__data__.id,first);d.querySelector('.ipe-domain-next').click();assert.equal(d.querySelectorAll('.ipe-trace-node').length,4);assert.equal(d.querySelector('.ipe-domain-next').disabled,true);}finally{dom.window.close();}
});
