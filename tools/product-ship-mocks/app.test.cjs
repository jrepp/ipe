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

test('systems view traces canonical source records to roles and contact profiles',async()=>{
  const dom=await page('board');
  try {
    const doc=dom.window.document;
    const context=data.provenance.contexts[0],record=context.records[0];
    assert.match(doc.querySelector('.provenance-summary').textContent,/ServiceNow/);
    assert.match(doc.querySelector('.provenance-summary').textContent,/Control owner/);
    doc.querySelector('[data-evidence-view="systems"]').click();
    const inspector=doc.querySelector('.provenance-inspector');
    for(const field of [record.instance,record.table,record.sys_id,record.required_state,record.observed_state,record.receipt]) assert.ok(inspector.textContent.includes(field));
    const assignment=context.assignments.find(item=>item.role==='Reviewer');
    [...doc.querySelectorAll('[data-prov-node]')].find(button=>button.dataset.provNode.startsWith('assignment:'+assignment.id+':')).click();
    assert.ok(doc.querySelector('.provenance-inspector').textContent.includes(assignment.person_id));
    [...doc.querySelectorAll('[data-prov-node]')].find(button=>button.dataset.provNode.startsWith('contact:'+assignment.person_id+':')).click();
    const person=context.people.find(item=>item.id===assignment.person_id);
    assert.ok(doc.querySelector('.provenance-inspector').textContent.includes(person.email.value));
    assert.ok(doc.querySelector('.provenance-inspector').textContent.includes(person.slack.user_id));
    assert.match(doc.querySelector('.provenance-workspace').textContent,/role does not prove approval/i);
    assert.equal(doc.querySelector('.decision-pill').textContent,'ALLOW');
  } finally {dom.window.close();}
});

test('unsafe contact and source URLs stay inert and source text is escaped',async()=>{
  const fixture=structuredClone(data),context=fixture.provenance.contexts[0];
  context.records[0].url='javascript:alert(1)';
  for(const person of context.people){
    person.name='<img src=x onerror=alert(1)>';
    person.email={...person.email,status:'available',value:'user@example.invalid?bcc=other@example.invalid'};
    person.slack={...person.slack,status:'available',url:'javascript:alert(1)'};
  }
  const dom=await page('board','',fixture);
  try {
    const doc=dom.window.document;
    doc.querySelector('[data-evidence-view="systems"]').click();
    assert.equal(doc.querySelector('.provenance-summary img'),null);
    for(const node of doc.querySelectorAll('a')) {
      assert.ok(!node.href.startsWith('javascript:'));
      assert.ok(!node.href.startsWith('mailto:'));
    }
    const person=context.people[0];
    [...doc.querySelectorAll('[data-prov-node]')].find(button=>button.dataset.provNode.startsWith('contact:'+person.id+':')).click();
    assert.equal(doc.querySelector('.contact-links a'),null);
    assert.equal(doc.querySelector('.provenance-inspector img'),null);
  } finally {dom.window.close();}
});

test('denied context stays explicit without hiding or changing the qualification result',async()=>{
  const fixture=structuredClone(data);
  fixture.provenance.contexts[0]={control_id:fixture.policies[0].controls[0].id,records:[],people:[],assignments:[],collection:{status:'unavailable',observed_at:fixture.as_of,receipt:null,gaps:[{code:'provider_read_failed'}]}};
  const dom=await page('board','',fixture);
  try {
    const doc=dom.window.document;
    doc.querySelector('[data-evidence-view="systems"]').click();
    assert.match(doc.querySelector('.provenance-empty').textContent,/provider_read_failed/);
    assert.doesNotMatch(doc.body.textContent,/\[object Object\]/);
    assert.equal(doc.querySelectorAll('[data-prov-node]').length,0);
    assert.equal(doc.querySelector('.decision-pill').textContent,'ALLOW');
  } finally {dom.window.close();}
});

test('stale roles and contact profiles remain labeled and cannot become current links',async()=>{
  const fixture=structuredClone(data),context=fixture.provenance.contexts[0];
  context.assignments[0].expires_at='2026-11-02T11:00:00Z';
  for(const person of context.people) {
    person.email.expires_at='2026-11-02T11:00:00Z';
    person.slack.observed_at='2026-11-03T00:00:00Z';
  }
  const dom=await page('board','?view=systems',fixture);
  try {
    const doc=dom.window.document;
    assert.match(doc.querySelector('.provenance-summary').textContent,/Expired/);
    assert.match(doc.querySelector('.provenance-summary').textContent,/Future observation/);
    assert.equal(doc.querySelector('.summary-contacts a'),null);
    [...doc.querySelectorAll('[data-prov-node]')].find(button=>button.dataset.provNode.startsWith('contact:')).click();
    assert.equal(doc.querySelector('.contact-links a'),null);
    assert.match(doc.querySelector('.provenance-inspector').textContent,/Not verified as current/);
  } finally {dom.window.close();}
});

test('duplicate identities and unresolved references do not choose a person by name or array order',async()=>{
  const fixture=structuredClone(data),context=fixture.provenance.contexts[0];
  context.people.push({...context.people[0],name:'Another matching identifier'});
  context.assignments[1].person_id='missing-person';
  context.assignments[1].resolution='unresolved';
  const dom=await page('board','?view=systems',fixture);
  try {
    const doc=dom.window.document;
    assert.match(doc.querySelector('.provenance-summary').textContent,/Person ID ambiguous/);
    assert.match(doc.querySelector('.provenance-summary').textContent,/Person unresolved/);
    assert.match(doc.querySelector('.provenance-relations').textContent,/person ID is ambiguous/);
    assert.match(doc.querySelector('.provenance-relations').textContent,/person unresolved/);
  } finally {dom.window.close();}
  fixture.provenance.contexts.push(structuredClone(context));
  const duplicate=await page('board','?view=systems',fixture);
  try {
    assert.match(duplicate.window.document.querySelector('.provenance-empty').textContent,/ambiguous/);
    assert.equal(duplicate.window.document.querySelectorAll('[data-prov-node]').length,0);
  } finally {duplicate.window.close();}
});

test('agent activity distinguishes unique agents, repeated calls, and qualification outcomes',async()=>{
  const dom=await page('board','?view=agents');
  try {
    const doc=dom.window.document;
    assert.match(doc.querySelector('.agent-activity-bar').textContent,/6 agents/);
    assert.match(doc.querySelector('.agent-summary').textContent,/2 querying agents/);
    assert.match(doc.querySelector('.agent-summary').textContent,/1 qualifying agent/);
    assert.match(doc.querySelector('.agent-summary').textContent,/2 agents active/);
    assert.equal(doc.querySelectorAll('.agent-ledger tbody tr').length,3);
    assert.match(doc.querySelector('.agent-baseline').textContent,/Model preparation, counted once/);
    scenario(dom,'blocked');
    assert.match(doc.querySelector('.agent-summary').textContent,/0 qualifying agents/);
    assert.match(doc.querySelector('.agent-ledger').textContent,/completed · not_met/);
    assert.equal(doc.querySelector('.decision-pill').textContent,'DENY');
    scenario(dom,'missing');
    assert.match(doc.querySelector('.agent-ledger').textContent,/completed · unknown/);
    assert.match(doc.querySelector('.agent-ledger').textContent,/No evidence record/);
    scenario(dom,'ready');
    doc.querySelector('[data-agent-evidence]').click();
    assert.equal(doc.querySelector('[data-evidence-view="record"]').getAttribute('aria-pressed'),'true');
    assert.match(doc.querySelector('.detail-card').textContent,/Evidence|EVIDENCE/);
  } finally {dom.window.close();}
});

test('query-only steps remain unqualified despite met policy evidence',async()=>{
  const dom=await page('board','?view=agents&policy=ship-operations&control=ship.operations.support-owner');
  try {
    const doc=dom.window.document;
    assert.match(doc.querySelector('.agent-summary').textContent,/1 querying agent/);
    assert.match(doc.querySelector('.agent-summary').textContent,/0 qualifying agents/);
    assert.equal(doc.querySelectorAll('.agent-ledger tbody tr').length,1);
    assert.doesNotMatch(doc.querySelector('.agent-ledger tbody').textContent,/Qualification/);
    assert.equal(doc.querySelector('.decision-pill').textContent,'ALLOW');
  } finally {dom.window.close();}
});

test('missing and zero baselines suppress savings; negative savings remain additional usage',async()=>{
  for(const variant of ['missing','zero','negative']) {
    const fixture=structuredClone(data),comparison=fixture.agent_activity.scenarios[0].comparison;
    if(variant==='missing') comparison.baseline=null;
    else if(variant==='zero') comparison.baseline={input_tokens:0,output_tokens:0,total_tokens:0,elapsed_ms:0,work_ms:0};
    else comparison.baseline={input_tokens:10,output_tokens:10,total_tokens:20,elapsed_ms:10,work_ms:10};
    const dom=await page('board','?view=agents',fixture);
    try {
      const doc=dom.window.document;
      assert.equal(doc.querySelector('.load-error'),null);
      if(variant==='negative') {
        assert.match(doc.querySelector('.agent-activity-bar').textContent,/tokens more/);
        assert.match(doc.querySelector('.agent-activity-bar').textContent,/slower/);
      } else {
        assert.match(doc.querySelector('.agent-baseline-unknown').textContent,/unavailable/);
        assert.match(doc.querySelector('.agent-activity-bar').textContent,/Unknown/);
        assert.doesNotMatch(doc.querySelector('.agent-activity-bar').textContent,/fewer|faster/);
      }
    } finally {dom.window.close();}
  }
});

test('agent activity is unavailable after evidence, policy, or event scope changes',async()=>{
  for(const variant of ['evidence','policy','event']) {
    const fixture=structuredClone(data);
    if(variant==='evidence') fixture.evidence[0].status='not_met';
    if(variant==='policy') fixture.policies[0].controls[0].description='A changed requirement';
    if(variant==='event') fixture.agent_activity.scenarios[0].events[0].artifact='different-artifact';
    const dom=await page('board','?view=agents',fixture);
    try {
      const doc=dom.window.document;
      assert.match(doc.querySelector('.agent-activity-bar').textContent,/unavailable/);
      assert.match(doc.querySelector('.agent-activity-empty').textContent,/No matching agent activity/);
      assert.equal(doc.querySelector('.agent-ledger'),null);
    } finally {dom.window.close();}
  }
});

test('chain report counts recorded runs, keeps policy gates separate, and opens findings',async()=>{
  const dom=await page('board','?report=usage');
  try {
    const doc=dom.window.document;
    assert.equal(doc.querySelector('.chain-report').open,true);
    assert.match(doc.querySelector('.chain-report summary').textContent,/5 simulated runs · 6 agents/);
    assert.equal(doc.querySelectorAll('.chain-run-history tbody tr').length,5);
    assert.match(doc.querySelector('.chain-report-body').textContent,/19 calls · 18 controls/);
    assert.match(doc.querySelector('.chain-report-body').textContent,/15 checks · 15 passed · 0 failed · 0 unknown/);
    doc.querySelector('[data-report-scenario="blocked"]').click();
    assert.match(doc.querySelector('.chain-report-body').textContent,/15 checks · 14 passed · 1 failed · 0 unknown/);
    assert.match(doc.querySelector('.chain-report-body').textContent,/17 met · 1 not met · 0 unknown/);
    doc.querySelector('[data-report-control]').click();
    assert.match(doc.querySelector('.agent-ledger').textContent,/completed · not_met/);
    doc.querySelector('[data-report-scenario="missing"]').click();
    assert.match(doc.querySelector('.chain-report-body').textContent,/15 checks · 14 passed · 0 failed · 1 unknown/);
    assert.match(doc.querySelector('.chain-report-body').textContent,/17 met · 0 not met · 1 unknown/);
    assert.match(doc.querySelector('.chain-report summary').textContent,/5 simulated runs/);
  } finally {dom.window.close();}
});

test('chain report starts closed and excludes stale runs from history',async()=>{
  const fixture=structuredClone(data);
  fixture.agent_activity.scenarios[1].events[0].artifact='stale';
  const dom=await page('board','',fixture);
  try {
    const doc=dom.window.document;
    assert.equal(doc.querySelector('.chain-report').open,false);
    assert.match(doc.querySelector('.chain-report summary').textContent,/4 simulated runs/);
    assert.equal(doc.querySelector('[data-report-scenario="blocked"]'),null);
    assert.match(doc.querySelector('.chain-cost').textContent,/Dollar cost is unavailable/);
    assert.match(doc.querySelector('.chain-cost').textContent,/excludes peak context-window use/);
  } finally {dom.window.close();}
});
