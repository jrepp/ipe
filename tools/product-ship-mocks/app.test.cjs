const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {JSDOM}=require('../../docs/node_modules/jsdom');
const {run}=require('../../testing/integration/hosted/product-ship/runner.cjs');
const data=run();
const app=fs.readFileSync(path.join(__dirname,'app.js'),'utf8');
const evaluator=fs.readFileSync(path.join(__dirname,'../../testing/integration/hosted/product-ship/product-ship.js'),'utf8');

async function page(name,query='',response=data) {
  const html=fs.readFileSync(path.join(__dirname,name+'.html'),'utf8');
  const dom=new JSDOM(html,{url:'http://127.0.0.1:8768/'+name+'.html'+query,runScripts:'outside-only'});
  dom.window.fetch=async()=>({ok:true,json:async()=>structuredClone(response)});
  dom.window.eval(evaluator);
  dom.window.eval(app);
  await new Promise(resolve=>setImmediate(resolve));
  return dom;
}
function scenario(dom,value) {
  const select=dom.window.document.querySelector('#scenario');
  select.value=value;
  select.dispatchEvent(new dom.window.Event('change',{bubbles:true}));
}

test('three views show the same five decisions and scenario-specific evidence',async()=>{
  for (const name of ['board','network','console']) {
    const dom=await page(name);
    try {
      const doc=dom.window.document;
      for (const [value,decision] of [['ready','ALLOW'],['blocked','DENY'],['missing','INDETERMINATE'],['expired','INDETERMINATE'],['wrong-artifact','INDETERMINATE']]) {
        scenario(dom,value);
        const decisionNode=doc.querySelector(name==='console'?'.result-main h2':'.decision-pill');
        assert.equal(decisionNode.textContent,decision,name+': '+value);
        const detail=doc.querySelector('.detail-card').textContent;
        if(value==='missing') assert.match(detail,/No evidence record/);
        if(value==='expired') assert.match(detail,/2026-11-02T11:59:59.000Z/);
        if(value==='wrong-artifact') assert.ok(detail.includes('sha256:'+'0'.repeat(64)));
        assert.doesNotMatch(doc.body.textContent,/\[object Object\]/);
      }
    } finally {dom.window.close();}
  }
});

test('readiness totals include all controls when a different child policy is selected',async()=>{
  const dom=await page('board','?scenario=blocked');
  try {
    const doc=dom.window.document;
    assert.match(doc.querySelector('.readiness-strip').textContent,/17\/18/);
    doc.querySelector('[data-policy="ship-security"]').click();
    assert.match(doc.querySelector('.readiness-strip').textContent,/17\/18/);
    assert.match(doc.querySelector('.selected-control').textContent,/Security review is complete/);
  } finally {dom.window.close();}
});

test('policy, control, and latest scenario survive navigation between views',async()=>{
  const dom=await page('board');
  let next;
  try {
    const doc=dom.window.document;
    doc.querySelector('[data-policy="ship-security"]').click();
    doc.querySelector('[data-control="ship.security.vulnerability-scan"]').click();
    scenario(dom,'expired');
    scenario(dom,'wrong-artifact');
    next=new URL(doc.querySelector('.top-links a[href*="network.html"]').href);
    assert.equal(next.searchParams.get('policy'),'ship-security');
    assert.equal(next.searchParams.get('control'),'ship.security.vulnerability-scan');
    assert.equal(next.searchParams.get('scenario'),'wrong-artifact');
  } finally {dom.window.close();}
  const target=await page('network',next.search);
  try {
    const doc=target.window.document;
    assert.equal(doc.querySelector('.network-inspector h2').textContent,'Security');
    assert.match(doc.querySelector('.network-control.selected').textContent,/Dependency scan/);
    assert.equal(doc.querySelector('#scenario').value,'wrong-artifact');
  } finally {target.window.close();}
});

test('invalid data shows an error instead of a ready release',async()=>{
  const dom=await page('board','',{schema:'unsupported'});
  try {
    assert.match(dom.window.document.querySelector('.load-error').textContent,/unsupported schema/);
    assert.equal(dom.window.document.querySelector('.decision-pill'),null);
  } finally {dom.window.close();}
});

test('combined checklist expands policies and retains selection when evidence fills the workspace',async()=>{
  const dom=await page('board');
  try {
    const doc=dom.window.document;
    assert.equal(doc.querySelectorAll('.chain-checklist').length,1);
    assert.equal(doc.querySelector('#group-ship-security').hidden,true);
    doc.querySelector('[data-chain-action="expand"]').click();
    assert.equal(doc.querySelectorAll('.chain-group [hidden]').length,0);
    doc.querySelector('[data-control="ship.security.vulnerability-scan"]').click();
    assert.equal(doc.activeElement.dataset.control,'ship.security.vulnerability-scan');
    doc.querySelector('[data-focus-evidence]').click();
    assert.ok(doc.querySelector('.chain-page').classList.contains('evidence-focused'));
    assert.match(doc.querySelector('.selected-control').textContent,/Dependency scan/);
    assert.equal(doc.activeElement.hasAttribute('data-focus-evidence'),true);
    doc.querySelector('[data-focus-evidence]').click();
    assert.ok(!doc.querySelector('.chain-page').classList.contains('evidence-focused'));
    assert.equal(doc.querySelector('.chain-control.chosen').dataset.control,'ship.security.vulnerability-scan');
    doc.querySelector('[data-chain-action="collapse"]').click();
    assert.equal(doc.querySelectorAll('.chain-group [hidden]').length,9);
  } finally {dom.window.close();}
});

test('evidence objects and connections expose mismatch details without rewriting source values',async()=>{
  const dom=await page('board','?scenario=wrong-artifact');
  try {
    const doc=dom.window.document;
    assert.equal(doc.querySelectorAll('[data-object]').length,6);
    doc.querySelector('[data-object="artifact"]').click();
    assert.match(doc.querySelector('.object-inspector').textContent,/Artifact differs/);
    assert.ok(doc.querySelector('.object-inspector').textContent.includes(data.release.artifact));
    assert.ok(doc.querySelector('.object-inspector').textContent.includes('sha256:'+'0'.repeat(64)));
    assert.equal(doc.activeElement.dataset.object,'artifact');
    doc.querySelector('[data-link="4"]').click();
    assert.match(doc.querySelector('.object-inspector').textContent,/names another artifact/);
    assert.equal(doc.querySelector('[data-link="4"]').getAttribute('aria-pressed'),'true');
    doc.querySelector('[data-evidence-view="record"]').click();
    assert.equal(doc.querySelector('.evidence-canvas'),null);
    assert.ok(doc.querySelector('.detail-card').textContent.includes('sha256:'+'0'.repeat(64)));
    scenario(dom,'missing');
    assert.match(doc.querySelector('.detail-card').textContent,/No evidence record/);
    doc.querySelector('[data-evidence-view="map"]').click();
    assert.match(doc.querySelector('[data-object="source"]').textContent,/Source unavailable/);
    assert.match(doc.querySelector('[data-link="2"]').textContent,/needs evidence/);
    assert.equal(doc.querySelector('.raw-evidence pre'),null);
  } finally {dom.window.close();}
});

test('expanded policy application and raw evidence stay open during object exploration',async()=>{
  const dom=await page('board');
  try {
    const doc=dom.window.document;
    for(const details of doc.querySelectorAll('[data-disclosure]')) {
      details.open=true;
      details.dispatchEvent(new dom.window.Event('toggle'));
    }
    doc.querySelector('[data-object="policy"]').click();
    assert.equal(doc.querySelector('[data-disclosure="openApplication"]').open,true);
    assert.equal(doc.querySelector('[data-disclosure="openRaw"]').open,true);
    assert.match(doc.querySelector('.object-inspector').textContent,/Product accessibility lead/);
    assert.match(doc.querySelector('.raw-evidence').textContent,/receipt/);
  } finally {dom.window.close();}
});
