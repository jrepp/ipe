const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {JSDOM}=require('../../docs/node_modules/jsdom');
const model=require('./fleet-model.js');
const html=fs.readFileSync(__dirname+'/fleet.html','utf8');
function page(query=''){
  const dom=new JSDOM(html,{url:'http://127.0.0.1:8768/fleet.html'+query,runScripts:'outside-only'});
  dom.window.eval(fs.readFileSync(__dirname+'/fleet-model.js','utf8'));
  dom.window.eval(fs.readFileSync(__dirname+'/fleet.js','utf8'));
  return dom;
}
test('fleet totals conserve policies and flow counts with bounded pages',()=>{
  const all=model.query();
  assert.equal(all.counts.total,350000);assert.equal(all.entityCount,3500);assert.equal(all.rows.length,25);
  assert.equal(all.counts.met+all.counts.not_met+all.counts.unknown+all.counts.error,350000);
  assert.equal(Object.values(all.edges).reduce((a,b)=>a+b,0),350000);
  assert.equal(all.groups.reduce((a,b)=>a+b,0),all.counts.new);
  assert.ok(all.counts.new>6500&&all.counts.new<7500);assert.ok(all.counts.recovered>3000&&all.counts.recovered<4000);
  for(const row of all.rows)assert.ok(row.evaluations>=100&&row.evaluations<=4800);
  const last=model.query({view:'policies',page:999999});assert.equal(last.rows.length,25);assert.equal(last.page,last.pages);
});
test('search and filters compose across entities and policies',()=>{
  const exact=model.query({q:'APP-000941',view:'policies'});assert.equal(exact.rowCount,1);assert.equal(exact.rows[0].id,'APP-000941');
  const entity=model.query({entity:'ENT-0010',view:'policies'});assert.equal(entity.counts.total,100);
  const failures=model.query({entity:'ENT-0010',transition:'new',change:'Source state',view:'policies'});assert.equal(failures.counts.total,4);assert.ok(failures.rows.every(r=>r.transition==='new'&&r.change===0));
  assert.equal(model.query({entity:'ENT-0010',team:'Commerce'}).counts.total,0);
  assert.equal(model.query({q:'does not exist'}).counts.total,0);
  assert.equal(model.query({entity:'ENT-9999'}).counts.total,0);
  const p1=model.query({view:'policies',transition:'new'}),p2=model.query({view:'policies',transition:'new',page:2});assert.equal(new Set([...p1.rows,...p2.rows].map(r=>r.id)).size,50);
});
test('causal replay retains failing inputs and does not invent a single cause for combined changes',()=>{
  for(let index=940;index<945;index++){
    const pair=model.comparison(index),snapshot=JSON.stringify(pair);
    assert.equal(pair.beforeResult,'met');assert.equal(pair.afterResult,'not_met');
    assert.equal(model.replay(index,'all').result,'met');
    assert.equal(JSON.stringify(model.comparison(index)),snapshot);
    if(index<944) assert.equal(model.replay(index,'0').result,'met');
  }
  assert.equal(model.replay(944,'0').result,'not_met');assert.equal(model.replay(944,'1').result,'not_met');
  assert.equal(model.comparison(980).available,false);assert.equal(model.replay(980,'all'),null);
  assert.equal(model.comparison(-1),null);assert.equal(model.replay(940,'8'),null);
  assert.equal(model.evaluate({approval:'approved'}),'unknown');
});
test('fleet navigation opens a failure comparison, replays it, and retains filters in the URL',()=>{
  const dom=page('?entity=ENT-0010&view=policies&transition=new');
  try{
    const doc=dom.window.document;
    assert.equal(doc.querySelectorAll('.fleet-results tbody tr').length,20);
    const link=[...doc.querySelectorAll('[data-policy-detail]')].find(a=>a.href.includes('APP-000945'));
    link.click();assert.match(dom.window.location.search,/entity=ENT-0010/);assert.equal(doc.querySelector('.fleet-detail').querySelectorAll('tr.changed').length,4);
    assert.match(doc.querySelector('.replay-result').textContent,/No replay/);
    doc.querySelector('[data-replay="0"]').click();assert.match(doc.querySelector('.replay-result').textContent,/Denied/);
    doc.querySelector('[data-replay="all"]').click();assert.match(doc.querySelector('.replay-result').textContent,/Passing/);
    doc.querySelector('[data-close]').click();assert.equal(doc.querySelector('.fleet-detail'),null);assert.equal(doc.querySelectorAll('.fleet-results tbody tr').length,20);
  }finally{dom.window.close();}
});
test('flow links filter the directory and search stays bounded and escaped',()=>{
  const dom=page();
  try{
    const doc=dom.window.document;
    doc.querySelector('.flow-alternatives a').click();assert.match(dom.window.location.search,/change=Source/);assert.match(dom.window.location.search,/transition=new/);
    assert.equal(doc.querySelectorAll('.fleet-results tbody tr').length,25);
    const search=doc.querySelector('#fleet-search');search.value='<img src=x onerror=alert(1)>';
    doc.querySelector('form').dispatchEvent(new dom.window.Event('submit',{bubbles:true,cancelable:true}));
    assert.equal(doc.querySelectorAll('img').length,0);assert.match(doc.querySelector('.fleet-results tbody').textContent,/No matches/);
    doc.querySelector('[data-clear]').click();assert.equal(doc.querySelectorAll('.fleet-results tbody tr').length,25);
  }finally{dom.window.close();}
});

test('shared policy definitions span consumers and all-flow paths retain the selected transition',()=>{
  const definitions=new Set();for(let e=0;e<model.ENTITY_COUNT;e++)for(let slot=0;slot<model.PER_ENTITY;slot++)definitions.add(model.record(e*100+slot).policy_id);
  assert.equal(definitions.size,500);
  const dom=page('?flow=all');
  try{
    const link=[...dom.window.document.querySelectorAll('.fleet-flow a')].find(a=>a.getAttribute('aria-label').includes('Passing to Denied'));
    link.dispatchEvent(new dom.window.MouseEvent('click',{bubbles:true,cancelable:true}));
    const params=Object.fromEntries(new URLSearchParams(dom.window.location.search));
    assert.equal(params.transition,'new');assert.equal(model.query(params).counts.total,model.query().counts.new);
  }finally{dom.window.close();}
});

test('all states are the default and users can leave new failures without stale filters',()=>{
  const dom=page('?entity=ENT-0010');
  try{
    const doc=dom.window.document;
    assert.equal(doc.querySelector('[data-flow="all"]').getAttribute('aria-pressed'),'true');
    doc.querySelector('[data-new-metric]').click();
    assert.equal(doc.querySelector('[data-flow="new"]').getAttribute('aria-pressed'),'true');
    assert.equal(new URLSearchParams(dom.window.location.search).get('transition'),'new');
    doc.querySelector('.flow-alternatives a').click();
    doc.querySelector('[data-flow="all"]').click();
    const params=Object.fromEntries(new URLSearchParams(dom.window.location.search));
    assert.equal(params.entity,'ENT-0010');
    assert.ok(!params.transition&&!params.status&&!params.change);
    assert.equal(model.query(params).counts.total,100);
    doc.querySelector('[data-clear]').click();
    assert.equal(doc.querySelector('[data-flow="all"]').getAttribute('aria-pressed'),'true');
    assert.equal(model.query(Object.fromEntries(new URLSearchParams(dom.window.location.search))).counts.total,350000);
  }finally{dom.window.close();}
});
