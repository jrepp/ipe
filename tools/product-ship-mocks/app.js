(() => {
  'use strict';

  const app = document.querySelector('#app');
  const mode = document.body.dataset.mode || 'home';
  const scenarios = [
    ['ready', 'Ready evidence'],
    ['blocked', 'Control not met'],
    ['missing', 'Evidence missing'],
    ['expired', 'Evidence expired'],
    ['wrong-artifact', 'Wrong artifact']
  ];
  const initialQuery = new URLSearchParams(location.search);
  const state = { scenario: initialQuery.get('scenario') || 'ready', policyId: initialQuery.get('policy') || '', controlId: initialQuery.get('control') || '', evidenceIndex: 0 };
  Object.assign(state, {openPolicies: new Set(), evidenceNode:'observation', evidenceLink:null, evidenceView:['map','record','systems'].includes(initialQuery.get('view'))?initialQuery.get('view'):'map', provenanceNode:'record', evidenceFocus:false, openApplication:false, openRaw:false});
  let data;

  const esc = (value) => String(value ?? '').replace(/[&<>"']/g, (c) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const text = (value, fallback = 'Not provided') => value == null || value === '' ? fallback : String(value);
  const uniqueMatch = (items, id) => {
    const matches = (items || []).filter((item) => item?.id === id);
    return matches.length === 1 ? matches[0] : null;
  };
  const provenanceFor = (controlId) => {
    const matches=(data.provenance?.contexts || []).filter((context) => context.control_id === controlId);
    return matches.length === 1 ? matches[0] : matches.length > 1 ? {ambiguous:true,control_id:controlId} : null;
  };
  const fieldText = (value) => {
    if (value == null || value === '') return 'Not provided';
    if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return String(value);
    return JSON.stringify(value);
  };
  const safeHref = (value, kind = 'https') => {
    if (typeof value !== 'string' || value.length > 2048 || /[\u0000-\u0020\u007f]/.test(value)) return null;
    if (kind === 'mailto') return /^[A-Za-z0-9.!$'*+^_`{|}~-]+@[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?)+$/.test(value) ? `mailto:${value}` : null;
    try {
      const url = new URL(value);
      if (url.protocol !== 'https:' || !url.hostname || url.username || url.password) return null;
      return url.href;
    } catch { return null; }
  };
  const contactIsCurrent = (item) => {
    if (!item || item.status !== 'available' || !item.observed_at || !item.expires_at || !data?.as_of) return false;
    const observed = Date.parse(item.observed_at), expires = Date.parse(item.expires_at), now = Date.parse(data.as_of);
    return Number.isFinite(observed) && Number.isFinite(expires) && Number.isFinite(now) && observed <= now && expires > now;
  };
  const sourceFreshness = (item) => {
    if (!item || item.status === 'unavailable' || item.resolution === 'unresolved') return 'Unavailable or unresolved';
    if (item.expires_at && data?.as_of && Date.parse(item.expires_at) <= Date.parse(data.as_of)) return 'Expired';
    if (item.observed_at && data?.as_of && Date.parse(item.observed_at) > Date.parse(data.as_of)) return 'Future observation';
    return contactIsCurrent({...item,status:item.status || 'available'}) ? 'Current source observation' : 'Not verified as current';
  };
  const stateLabel = (value) => typeof value === 'string' ? value : fieldText(value);
  const contextOwners = (context) => (context?.assignments || []).map((assignment) => {
    const person = assignment.resolution === 'resolved' ? uniqueMatch(context.people,assignment.person_id) : null;
    return {assignment, person};
  });
  const safeLink = (label, href, className = '') => href ? `<a class="${className}" href="${esc(href)}" target="_blank" rel="noopener noreferrer">${esc(label)}</a>` : `<span class="${className}">${esc(label)}</span>`;
  const rootPolicy = () => data.policy || data.root_policy || data.policies.find((p) => !p.parent_id) || { id: 'product-ship', title: 'Product ship policy', description: '' };
  const children = () => {
    const root = rootPolicy();
    const ids = root.children || [];
    return data.policies.filter((p) => ids.includes(p.id) || p.parent_id === root.id || (!ids.length && !p.parent_id));
  };
  const selectedPolicy = () => children().find((p) => p.id === state.policyId) || children()[0];
  const controls = (policy = selectedPolicy()) => policy?.controls || [];
  const selectedControl = () => controls().find((c) => c.id === state.controlId) || controls()[0];
  const resultReason = (result) => {
    const items = result.reasons || [];
    return items.map((item) => typeof item === 'string' ? item : item.message).filter(Boolean).join(' ');
  };
  const evaluate = () => {
    const evaluator = window.IpeProductShip?.evaluate;
    if (!evaluator) throw new Error('The shared evaluator did not load.');
    return evaluator(data, state.scenario);
  };
  const checkFor = (controlId, result = evaluate()) => (result.checks || []).find((c) => c.control_id === controlId) || {};
  const evidenceFor = (controlId) => {
    const evidence = checkFor(controlId).evidence;
    return evidence ? [evidence] : [];
  };
  const statusName = (status) => ({ met: 'Met', not_met: 'Not met', unknown: 'Unknown' }[status] || 'Unknown');
  const statusClass = (status) => ({ met: 'met', not_met: 'not-met', unknown: 'unknown' }[status] || 'unknown');
  const decisionName = (value) => ({ allow: 'ALLOW', deny: 'DENY', indeterminate: 'INDETERMINATE' }[value] || 'INDETERMINATE');
  const controlsStatus = (policy, result) => {
    const items = controls(policy).map((c) => checkFor(c.id, result));
    return { met: items.filter((c) => c.status === 'met').length, total: items.length, unknown: items.filter((c) => c.status === 'unknown').length, notMet: items.filter((c) => c.status === 'not_met').length };
  };
  const badge = (status) => `<span class="status-pill ${statusClass(status)}">${statusName(status)}</span>`;
  const decisionBadge = (decision) => `<span class="decision-pill ${statusClass(decision === 'allow' ? 'met' : decision === 'deny' ? 'not_met' : 'unknown')}">${decisionName(decision)}</span>`;
  const selectScenario = () => `<label class="scenario-control"><span>Scenario</span><select id="scenario">${scenarios.map(([value, label]) => `<option value="${value}" ${state.scenario === value ? 'selected' : ''}>${label}</option>`).join('')}</select></label>`;
  const releaseBlock = () => `<div class="release-inline"><span class="release-glyph">↗</span><div><small>RELEASE CANDIDATE</small><strong>${esc(data.release.name)} <span>${esc(data.release.version)}</span></strong></div><div class="release-target"><small>ARTIFACT</small><b>${esc(data.release.artifact)}</b></div><div class="release-target"><small>ENVIRONMENT</small><b>${esc(data.release.environment)}</b></div></div><p class="simulation-clock">Simulated evaluation time: <time datetime="${esc(data.as_of)}">${esc(data.as_of)}</time></p>`;
  const crumb = (policy, control) => `<nav class="crumb" aria-label="Policy breadcrumb"><a href="index.html">${esc(rootPolicy().title)}</a>${policy ? `<span aria-hidden="true">›</span><button data-policy="${esc(policy.id)}">${esc(policy.title)}</button>` : ''}${control ? `<span aria-hidden="true">›</span><span aria-current="page">${esc(control.title)}</span>` : ''}</nav>`;
  const evidenceDetail = (control, check) => {
    const evidences = check?.evidence ? [check.evidence] : [];
    const entry = evidences[state.evidenceIndex] || evidences[0];
    if (!entry) return `<section class="detail-card"><p class="eyebrow">EVIDENCE RECORD</p><h3>No evidence record</h3><p>The sample has no evidence for this control and release. The simulated result is unknown.</p></section>`;
    return `<section class="detail-card"><div class="detail-title"><div><p class="eyebrow">EVIDENCE RECORD</p><h3>${esc(text(entry.receipt, 'Evidence receipt'))}</h3></div>${badge(check.status)}</div>
      <p class="detail-reason">${esc(text(check.reason, entry.reason || 'No reason was supplied.'))}</p>
      <dl class="facts-grid"><div><dt>Observed</dt><dd>${esc(text(entry.observed_at))}</dd></div><div><dt>Expires</dt><dd>${esc(text(entry.expires_at))}</dd></div><div><dt>Source</dt><dd>${esc(text(entry.source))}</dd></div><div><dt>Artifact</dt><dd>${esc(text(entry.artifact))}</dd></div><div><dt>Environment</dt><dd>${esc(text(entry.environment))}</dd></div><div><dt>Release</dt><dd>${esc(text(entry.release_id))}</dd></div></dl>
      ${entry.source ? `<div class="source-link">Source record · ${esc(entry.source)}</div>` : ''}</section>`;
  };

  const provenanceSummary = (context) => {
    if (!context) return `<section class="provenance-summary" aria-label="Provenance and people"><div><strong>Systems and people</strong><p>No provenance context is available for this control.</p></div><span class="simulation-caveat">Sample data · no real ServiceNow or Slack account</span></section>`;
    if (context.ambiguous) return `<section class="provenance-summary" aria-label="Provenance and people"><div><strong>Systems and people</strong><p>More than one provenance context uses this control ID. The UI cannot select one source safely.</p></div><span class="simulation-caveat">Sample data · source IDs are ambiguous</span></section>`;
    const records = context.records || [];
    const assignments = contextOwners(context);
    const roleSummary = ['Control owner','Reviewer','Approver'].map((role) => {
      const entries = assignments.filter(({assignment}) => assignment.role === role);
      const names = entries.map(({assignment,person}) => {
        const personMatches=(context.people || []).filter((item)=>item.id===assignment.person_id).length;
        const label=person ? (person.name || person.id) : assignment.resolution !== 'resolved' ? 'Person unresolved' : personMatches>1 ? 'Person ID ambiguous' : 'Person record unavailable';
        return `${label} · ${sourceFreshness(assignment)}`;
      });
      return `<div><dt>${esc(role)}${entries.length > 1 ? 's' : ''}</dt><dd>${names.length ? names.map(esc).join(', ') : 'Not assigned'}</dd></div>`;
    }).join('');
    const people = [...new Map(assignments.filter(({person})=>person).map(({person})=>[person.id,person])).values()];
    const contacts = people.length ? people.map((person) => {
      const email=person.email || {}, slack=person.slack || {};
      const emailHref=contactIsCurrent(email)?safeHref(email.value,'mailto'):null;
      const slackHref=contactIsCurrent(slack)?safeHref(slack.url):null;
      const detail=(label,item,href,value)=>`${safeLink(value || label,href)} <small>${esc(sourceFreshness(item))}</small>`;
      return `<span class="contact-summary"><b>${esc(person.name || person.id)}</b><span>${detail('Email',email,emailHref,email.value)} · ${detail('Slack profile',slack,slackHref,slack.display_name || slack.user_id)}</span></span>`;
    }).join('') : '<span class="contact-summary">No resolved people or contact details</span>';
    const firstRecord = records.length===1 ? records[0] : null;
    const recordLabel = firstRecord ? `${firstRecord.system || 'External system'} · ${firstRecord.table || 'table'} · ${firstRecord.number || 'display number unavailable'} · ${sourceFreshness(firstRecord)}` : records.length>1 ? `${records.length} canonical records · inspect source IDs` : 'No linked record';
    const recordUrl = firstRecord?.status === 'available' ? safeHref(firstRecord.url) : null;
    const gapText=(context.collection?.gaps || []).map((gap)=>typeof gap==='string'?gap:gap?.message || gap?.code || fieldText(gap)).join('; ') || 'None reported';
    return `<section class="provenance-summary" aria-label="Provenance and people"><div class="provenance-summary-main"><div><strong>Systems &amp; people</strong><p>${safeLink(recordLabel,recordUrl)}</p><button class="summary-explore" data-open-provenance>Inspect source records and roles</button></div><dl>${roleSummary}</dl></div><details class="summary-contacts"><summary>Contact profiles · ${people.length} people</summary><div class="summary-contact-list">${contacts}</div></details><div class="provenance-summary-foot"><span>${records.length} canonical record${records.length===1?'':'s'} · ${assignments.length} role assignment${assignments.length===1?'':'s'} · ${esc(text(context.collection?.status,'Unavailable'))}</span><span class="simulation-caveat">Simulated source data · roles do not prove approval</span>${context.collection?.gaps?.length?`<span>Gaps: ${esc(gapText)}</span>`:''}</div></section>`;
  };

  const provenanceWorkspace = (context) => {
    if (!context) return `<section class="provenance-empty" aria-label="Systems and people"><h3>No provenance context</h3><p>This sample has no source records or people for the selected control. This does not change the evaluator result.</p></section>`;
    if (context.ambiguous) return `<section class="provenance-empty" aria-label="Systems and people"><h3>Provenance context is ambiguous</h3><p>More than one source context uses this control ID. The UI does not choose a context by order.</p><p>This data issue does not change the evaluator result.</p></section>`;
    const records = context.records || [], assignments = context.assignments || [], people = context.people || [];
    const objects = [];
    records.forEach((record,index)=>objects.push({id:`record:${record.id}:${index}`,kind:'ServiceNow record',title:record.title || record.number || record.sys_id || `Record ${index+1}`,source:record}));
    assignments.forEach((assignment,index)=>objects.push({id:`assignment:${assignment.id}:${index}`,kind:'Role assignment',title:assignment.role || `Assignment ${index+1}`,source:assignment}));
    people.forEach((person,index)=>{
      objects.push({id:`person:${person.id}:${index}`,kind:'Person',title:person.name || person.id || `Person ${index+1}`,source:person});
      objects.push({id:`contact:${person.id}:${index}`,kind:'Contact details',title:person.name || person.id || `Contact ${index+1}`,source:person});
    });
    if (!objects.length) return `<section class="provenance-empty" aria-label="Systems and people"><h3>No source records or role assignments</h3><p>Collection status: ${esc(text(context.collection?.status,'Unavailable'))}. ${esc((context.collection?.gaps || []).map((gap)=>typeof gap==='string'?gap:gap?.message || gap?.code || fieldText(gap)).join('; '))}</p><p>This gap does not change the evaluator result.</p></section>`;
    const active=objects.find((item)=>item.id===state.provenanceNode) || objects[0];
    const objectButton=(item)=>{
      const group=item.kind==='ServiceNow record'?records:item.kind==='Role assignment'?assignments:people;
      const duplicate=group.filter((entry)=>entry.id===item.source.id).length>1;
      return `<button class="provenance-object ${item.id===active.id?'active':''} ${item.source.status==='unavailable'||item.source.resolution==='unresolved'||duplicate?'uncertain':''}" data-prov-node="${esc(item.id)}" aria-pressed="${item.id===active.id}"><span>${esc(item.kind)}</span><strong>${esc(item.title)}</strong>${duplicate?'<small>Duplicate source ID · references are ambiguous</small>':''}</button>`;
    };
    const recordFor = (assignment) => uniqueMatch(records,assignment.source_record_id);
    const personFor = (assignment) => assignment.resolution==='resolved' ? uniqueMatch(people,assignment.person_id) : null;
    const relations = [];
    assignments.forEach((assignment)=>{
      const record=recordFor(assignment), person=personFor(assignment);
      const recordMatches=records.filter((item)=>item.id===assignment.source_record_id).length;
      const personMatches=people.filter((item)=>item.id===assignment.person_id).length;
      const recordLabel=record?`${record.title || record.number || record.sys_id} → `:recordMatches>1?'Ambiguous source record ID → ':'Source record reference unavailable → ';
      const personLabel=person?(person.name || person.id):assignment.resolution!=='resolved'?'person unresolved':personMatches>1?'person ID is ambiguous':'person record unavailable';
      relations.push(`<li>${esc(recordLabel)}${esc(assignment.role || 'Role')} → ${esc(personLabel)}. Source record ID: ${esc(text(assignment.source_record_id))}; person ID: ${esc(text(assignment.person_id))}.</li>`);
    });
    const values = (item) => {
      const value=item.source, fields={};
      if(item.kind==='ServiceNow record') Object.assign(fields,{'System':value.system,'Instance':value.instance,'Table':value.table,'Canonical sys_id':value.sys_id,'Display number':value.number,'Required state':stateLabel(value.required_state),'Observed state':stateLabel(value.observed_state),'Record relation':value.relation,'Record status':value.status,'Source revision':value.source_revision,'Broker read receipt':value.receipt,'Source receipt':value.source_receipt,'Collected at':value.observed_at,'Expires at':value.expires_at,'Record URL':safeHref(value.url) || 'Unavailable or unsafe URL'});
      if(item.kind==='Role assignment') {const person=personFor(value), record=recordFor(value); Object.assign(fields,{'Role':value.role,'Assignment status':sourceFreshness(value),'Person resolution':person?'Resolved':value.resolution==='resolved'&&people.filter((person)=>person.id===value.person_id).length>1?'Ambiguous person ID':value.resolution,'Person ID reference':value.person_id,'Resolved person':person?.name,'Source record ID':value.source_record_id,'Record reference status':record?'Resolved':records.filter((record)=>record.id===value.source_record_id).length>1?'Ambiguous record ID':'Unavailable','Referenced record':record?.title || record?.number || record?.sys_id,'Source observed at':value.observed_at,'Source expires at':value.expires_at});}
      if(item.kind==='Person') Object.assign(fields,{'Name':value.name,'Title':value.title,'Directory system ID':value.system_id,'Person ID':value.id});
      if(item.kind==='Contact details') {
        const email=value.email || {}, slack=value.slack || {};
        const contactStatus=(contact)=>contact.status!=='available'?'Unavailable':contactIsCurrent(contact)?'Current source observation':'Not verified as current';
        Object.assign(fields,{'Email':email.value,'Email status':contactStatus(email),'Email source':email.source,'Email observed at':email.observed_at,'Email expires at':email.expires_at,'Slack display name':slack.display_name,'Slack user ID':slack.user_id,'Slack workspace':slack.workspace,'Slack status':contactStatus(slack),'Slack source':slack.source,'Slack observed at':slack.observed_at,'Slack expires at':slack.expires_at,'Slack profile URL':safeHref(slack.url) || 'Unavailable or unsafe URL'});
      }
      return fields;
    };
    const activeFields=values(active);
    const stateComparison=active.kind==='ServiceNow record'?`<div class="state-comparison"><div><b>Required state</b><span>${esc(stateLabel(active.source.required_state))}</span></div><div><b>Observed state</b><span>${esc(stateLabel(active.source.observed_state))}</span></div></div>`:'';
    const email=active.kind==='Contact details'?active.source.email:null, slack=active.kind==='Contact details'?active.source.slack:null;
    const contactLinks=active.kind==='Contact details'?`<div class="contact-links">${safeLink('Email',contactIsCurrent(email)?safeHref(email.value,'mailto'):null)} ${safeLink('Slack profile',contactIsCurrent(slack)?safeHref(slack.url):null)}</div>`:'';
    return `<section class="provenance-workspace" aria-label="ServiceNow records and people"><div class="provenance-notice"><strong>Simulated integrations</strong><span>These sample records do not connect to real ServiceNow or Slack accounts. A role does not prove an approval action.</span></div><div class="provenance-collection"><b>Source collection</b><span>${esc(text(context.collection?.status,'Unavailable'))}</span><span>Collected ${esc(text(context.collection?.observed_at))}</span><span>Receipt ${esc(text(context.collection?.receipt))}</span><span>Gaps: ${esc((context.collection?.gaps || []).map((gap)=>typeof gap==='string'?gap:gap?.message || gap?.code || fieldText(gap)).join('; ') || 'None reported')}</span></div><div class="provenance-map"><aside class="provenance-navigation" aria-label="Source objects"><div class="provenance-objects">${objects.map(objectButton).join('')}</div><details class="provenance-relations"><summary>Source relationships</summary><ul>${relations.length?relations.join(''):'<li>No role assignment relation is available.</li>'}</ul><p>Each link uses the source IDs shown above. The view does not match people by name.</p></details></aside><section class="object-inspector detail-card provenance-inspector" aria-label="Selected provenance object"><div class="object-heading"><div><p class="eyebrow">${esc(active.kind)}</p><h3>${esc(active.title)}</h3></div></div><p>${esc(active.kind==='Role assignment'?'This role and person link come from the listed source IDs. They do not record an approval action.':active.kind==='Contact details'?'Contact details are source observations. The UI marks a contact current only when its observation time and expiry support that label.':'Values below come from the sample source record. Required and observed states remain separate.')}</p>${stateComparison}${Object.entries(activeFields).filter(([key])=>!['Required state','Observed state'].includes(key)).map(([key,value])=>`<div class="provenance-field"><b>${esc(key)}</b><span>${esc(fieldText(value))}</span></div>`).join('')}${contactLinks}</section></div></section>`;
  };
  const setPolicy = (id) => {
    if (mode==='board' && state.policyId===id && state.openPolicies.has(id)) state.openPolicies.delete(id);
    else state.openPolicies.add(id);
    state.policyId = id; state.controlId = ''; state.evidenceIndex = 0; state.evidenceLink=null; render();
    if (mode==='board') [...document.querySelectorAll('.chain-policy')].find(button=>button.dataset.policy===id)?.focus();
  };
  const bind = () => {
    document.querySelectorAll('.central-icon,.central-arrow,.row-mark,.control-symbol,.trace-number,.map-node span').forEach((element) => element.setAttribute('aria-hidden', 'true'));
    const query = new URLSearchParams();
    if (state.scenario !== 'ready') query.set('scenario', state.scenario);
    if (state.policyId) query.set('policy', state.policyId);
    if (state.controlId) query.set('control', state.controlId);
    if (state.evidenceView!=='map') query.set('view',state.evidenceView);
    document.querySelectorAll('a[href]').forEach((link) => {
      const target = link.getAttribute('href').split('?')[0];
      if (!['index.html','board.html','network.html','console.html'].includes(target)) return;
      link.href = query.size ? `${target}?${query.toString()}` : target;
    });
    document.querySelector('#scenario')?.addEventListener('change', (event) => { state.scenario = event.target.value; render(); document.querySelector('#scenario')?.focus(); });
    document.querySelectorAll('[data-policy]:not([data-control])').forEach((button) => button.addEventListener('click', () => setPolicy(button.dataset.policy)));
    document.querySelectorAll('[data-control]').forEach((button) => button.addEventListener('click', () => { state.policyId = button.dataset.policy || state.policyId; state.controlId = button.dataset.control; state.evidenceIndex = 0; state.evidenceLink=null; render(); if(mode==='board') [...document.querySelectorAll('[data-control]')].find(item=>item.dataset.control===state.controlId)?.focus(); }));
    document.querySelectorAll('[data-evidence-index]').forEach((button) => button.addEventListener('click', () => { state.evidenceIndex = Number(button.dataset.evidenceIndex); render(); }));
  };

  function home() {
    const result = evaluate();
    const counts = children().reduce((sum, p) => { const s = controlsStatus(p, result); sum.total += s.total; sum.met += s.met; sum.unknown += s.unknown; sum.notMet += s.notMet; return sum; }, {total:0, met:0,unknown:0,notMet:0});
    app.innerHTML = `<section class="home-hero"><div class="hero-copy"><p class="eyebrow">PRODUCT SHIP QUALIFICATION</p><h1>One release.<br><span>Every policy in view.</span></h1><p>Review the central ship policy and its linked controls for accessibility, security, open-source legal, and enterprise requirements.</p><div class="hero-actions"><a class="primary-link" href="board.html">Review release readiness <span aria-hidden="true">→</span></a><span class="simulation-note">SIMULATED ENFORCEMENT · FAIL CLOSED</span></div></div><div class="hero-stamp"><span>SHIP</span><b>POLICY</b><small>RELEASE GATE</small><div class="stamp-lines">${Array.from({length:5},() => '<i></i>').join('')}</div></div></section>
      ${releaseBlock()}<section class="overview-grid"><div class="overview-intro"><p class="eyebrow">CHOOSE A VIEW</p><h2>Explore the release gate</h2><p>Each view uses the same release data and simulated evaluator. Select a policy to inspect its controls and evidence.</p></div>
       <a class="view-card card-board" href="board.html"><span class="card-number">01 / READINESS</span><div class="card-visual"><div class="mini-gate"><i></i><i></i><i></i><b>${counts.met}/${counts.total}</b></div></div><h3>Release gate board <span>→</span></h3><p>Explore a compact policy checklist and a larger evidence workspace.</p></a>
       <a class="view-card card-network" href="network.html"><span class="card-number">02 / RELATIONSHIPS</span><div class="card-visual network-mini"><b>${esc(rootPolicy().title)}</b><i></i><i></i><i></i><small>${children().length} linked policies</small></div><h3>Policy network <span>→</span></h3><p>Move from the central policy into each child policy and control.</p></a>
       <a class="view-card card-console" href="console.html"><span class="card-number">03 / DECISION TRACE</span><div class="card-visual console-mini"><span>evaluate()</span><b>${decisionName(result.decision)}</b><i></i><i></i></div><h3>Enforcement console <span>→</span></h3><p>Change a scenario and inspect the shared evaluation trace.</p></a>
      </section><section class="home-summary"><div><small>SIMULATED RESULT</small>${decisionBadge(result.decision)}</div><p>${counts.met} met <span>·</span> ${counts.notMet} not met <span>·</span> ${counts.unknown} unknown</p>${selectScenario()}</section>`;
    bind();
  }

  function evidenceWorkspace(policy, control, check) {
    if (!control) return '<p class="empty-state">Select a control to explore its evidence.</p>';
    const evidence = check.evidence;
    const provenance = provenanceFor(control.id);
    const sameArtifact = evidence?.artifact === data.release.artifact;
    const nodes = [
      {id:'policy',kind:'Policy',title:policy.title,description:'The central ship policy requires this child policy. It lists this control.',fields:{'Policy ID':policy.id,'Parent policy':policy.parent_id,Owner:policy.owner,Requirement:control.description}},
      {id:'control',kind:'Control',title:control.title,description:'This authored control names the expected evidence source. This is a simulated application of that requirement.',fields:{'Control ID':control.id,'Expected source':control.source,'Required for release':control.required ? 'Yes' : 'No'}},
      {id:'check',kind:'Evidence check',title:statusName(check.status),description:check.reason,fields:{Result:statusName(check.status),'Evaluation time':data.as_of,'Expected release':data.release.id,'Expected artifact':data.release.artifact,'Expected environment':data.release.environment}},
      {id:'observation',kind:'Observation',title:evidence ? 'Reported control result' : 'No evidence record',description:evidence ? evidence.reason : 'No matching record is available. This gap cannot establish that the control passed.',fields:evidence ? {'Reported status':evidence.status,'Control ID':evidence.control_id,'Observed at':evidence.observed_at,'Expires at':evidence.expires_at,'Reported artifact':evidence.artifact,'Reported environment':evidence.environment,'Receipt':evidence.receipt} : {Status:'Missing evidence','Required control':control.id}},
      {id:'source',kind:'Source',title:evidence?.source || 'Source unavailable',description:evidence ? 'The synthetic collector retained this source label and read receipt. A receipt does not establish control effectiveness.' : 'No source record was retained for this check.',fields:evidence ? {Source:evidence.source,Receipt:evidence.receipt,'Expected source':control.source} : {'Expected source':control.source}},
      {id:'artifact',kind:'Release context',title:evidence ? (sameArtifact ? 'Artifact matches' : 'Artifact differs') : 'Context unverified',description:'The check compares the evidence release, artifact, and environment with the requested release context.',fields:{'Expected release':data.release.id,'Reported release':evidence?.release_id,'Expected artifact':data.release.artifact,'Reported artifact':evidence?.artifact,'Expected environment':data.release.environment,'Reported environment':evidence?.environment}}
    ];
    const links = [
      ['policy','control','requires','The policy lists this control by its explicit identifier.'],
      ['control','check','checked by','The demo evaluator checks this required control for the selected release.'],
      ['check','observation',evidence ? 'examines' : 'needs evidence',evidence ? 'The check reads the evidence record for this control and release. The result remains separate from the reported status.' : 'No evidence record connects to this check.'],
      ['observation','source',evidence ? 'collected from' : 'source missing',evidence ? 'The observation retains the source label and synthetic read receipt shown here.' : 'The missing observation provides no source or receipt.'],
      ['observation','artifact',evidence ? (sameArtifact ? 'names artifact' : 'names another artifact') : 'context missing','The source record names its own release context. A mismatch is retained, not replaced by the expected artifact.']
    ];
    const active = nodes.find(node=>node.id===state.evidenceNode) || nodes[3];
    const nodeButton = id => {
      const node=nodes.find(item=>item.id===id);
      return `<button class="evidence-object ${node.id===active.id?'active':''} ${(!evidence && ['observation','source','artifact'].includes(id)) || (id==='artifact'&&!sameArtifact) ? 'uncertain':''}" data-object="${id}" aria-pressed="${id===active.id}"><span>${esc(node.kind)}</span><strong>${esc(node.title)}</strong><small>${id==='check'?esc(check.reason):id==='observation'?(evidence?'Select to inspect the source record':'Missing · select to inspect'): 'Select to explore'}</small></button>`;
    };
    const relation = index => `<button class="evidence-relation" data-link="${index}" aria-pressed="${state.evidenceLink===index}" aria-label="Explain connection: ${esc(nodes.find(n=>n.id===links[index][0]).kind+' '+links[index][2]+' '+nodes.find(n=>n.id===links[index][1]).kind)}"><span>${esc(links[index][2])}</span><span aria-hidden="true">${index===3?'←':index===2?'↓':'→'}</span></button>`;
    const fields = values => `<dl class="object-fields">${Object.entries(values).map(([key,value])=>`<div><dt>${esc(key)}</dt><dd>${esc(text(value))}</dd></div>`).join('')}</dl>`;
    const selectedLink = state.evidenceLink===null ? null : links[state.evidenceLink];
    return `${provenanceSummary(provenance)}<div class="evidence-tabs" role="group" aria-label="Evidence view"><button data-evidence-view="map" aria-pressed="${state.evidenceView==='map'}">Connections</button><button data-evidence-view="record" aria-pressed="${state.evidenceView==='record'}">Evidence record</button><button data-evidence-view="systems" aria-pressed="${state.evidenceView==='systems'}">Systems &amp; people</button><span>Sample relationships · no enforcement event</span></div>
      ${state.evidenceView==='map' ? `<section class="evidence-canvas" aria-label="Control evidence connections"><div class="evidence-path">${nodeButton('policy')}${relation(0)}${nodeButton('control')}${relation(1)}${nodeButton('check')}</div><div class="evidence-branch"><svg viewBox="0 0 100 60" preserveAspectRatio="none" aria-hidden="true"><path d="M83.33 0 V20 H50 V58 M49 53 L50 58 L51 53"/></svg>${relation(2)}</div><div class="evidence-path evidence-sources">${nodeButton('source')}${relation(3)}${nodeButton('observation')}${relation(4)}${nodeButton('artifact')}</div><p class="map-help">Select an object or a connection to inspect it. Dashed borders mark missing or mismatched evidence.</p></section>
      <section class="object-inspector detail-card" aria-label="Selected evidence object"><div class="object-heading"><div><p class="eyebrow">${selectedLink?'CONNECTION DETAILS':esc(active.kind)}</p><h3>${selectedLink?esc(nodes.find(n=>n.id===selectedLink[0]).kind+' → '+nodes.find(n=>n.id===selectedLink[1]).kind):esc(active.title)}</h3></div><div class="inspector-result">Check result ${badge(check.status)}</div></div><p>${esc(selectedLink ? selectedLink[3] : active.description)}</p>${selectedLink ? fields({From:nodes.find(n=>n.id===selectedLink[0]).title,Relationship:selectedLink[2],To:nodes.find(n=>n.id===selectedLink[1]).title}) : fields(active.fields)}</section>` : state.evidenceView==='record' ? evidenceDetail(control,check) : provenanceWorkspace(provenance)}
      <details class="application-details" data-disclosure="openApplication" ${state.openApplication?'open':''}><summary>How this control applies to the release</summary><p>${esc(policy.description)}</p>${fields({'Central policy':rootPolicy().title,'Child policy':policy.title,'Control':control.title,'Owner':policy.owner,'Release':data.release.id,'Environment':data.release.environment})}<p>All child policies and their required controls must pass in this simulation. These links do not record a production enforcement event.</p></details>
      <details class="application-details raw-evidence" data-disclosure="openRaw" ${state.openRaw?'open':''}><summary>Full evidence record${evidence?'':' · unavailable'}</summary>${evidence ? `<pre>${esc(JSON.stringify(evidence,null,2))}</pre>` : '<p>No evidence record is available for this control and release.</p>'}</details>`;
  }

  function board() {
    const result=evaluate(), selected=selectedPolicy(), control=selectedControl();
    const check=control ? checkFor(control.id,result) : {};
    const met=result.checks.filter(item=>item.status==='met').length;
    const total=result.checks.length;
    app.innerHTML=`<div class="page-shell board-page chain-page ${state.evidenceFocus?'evidence-focused':''}"><div class="chain-heading"><div><p class="eyebrow">PRODUCT SHIP QUALIFICATION</p><h1>Release qualification</h1><p>${esc(data.release.name)} <strong>${esc(data.release.version)}</strong> <span>· ${esc(data.release.environment)}</span></p></div>${selectScenario()}</div>
      <section class="readiness-strip compact-readiness" aria-label="Release summary"><div><small>SIMULATED DECISION</small>${decisionBadge(result.decision)}</div><strong>${met}/${total} <span>controls met</span></strong><span class="release-summary-reason">${esc(resultReason(result)||'All required controls have matching evidence.')}</span><span class="evaluation-clock">Evaluated at <time datetime="${esc(data.as_of)}">${esc(data.as_of)}</time></span></section>
      <div class="chain-workspace"><aside class="chain-checklist" aria-label="Policy and control checklist"><div class="checklist-heading"><h2>Policy chain</h2><span>${children().length} policies · ${total} controls</span></div><a class="chain-root" href="index.html">${esc(rootPolicy().title)} <span>All required</span></a><div class="checklist-actions"><button data-chain-action="expand">Expand all</button><button data-chain-action="collapse">Collapse all</button></div>
      <div class="chain-groups">${children().map(policy=>{
        const summary=controlsStatus(policy,result),open=state.openPolicies.has(policy.id);
        return `<section class="chain-group"><button class="chain-policy ${selected?.id===policy.id?'selected':''}" data-policy="${esc(policy.id)}" aria-expanded="${open}" aria-controls="group-${esc(policy.id)}"><span aria-hidden="true">${open?'▾':'▸'}</span><strong>${esc(policy.title)}</strong><span>${summary.met}/${summary.total}</span></button><div id="group-${esc(policy.id)}" ${open?'':'hidden'}>${controls(policy).map(item=>{
          const itemCheck=checkFor(item.id,result);
          return `<button class="chain-control ${control?.id===item.id?'chosen':''}" data-policy="${esc(policy.id)}" data-control="${esc(item.id)}" ${control?.id===item.id?'aria-current="true"':''}><span class="check-mark ${statusClass(itemCheck.status)}" aria-label="${statusName(itemCheck.status)}">${itemCheck.status==='met'?'✓':itemCheck.status==='not_met'?'!':'?'}</span><span>${esc(item.title)}</span></button>`;
        }).join('')}</div></section>`;
      }).join('')}</div><p class="checklist-note">Status comes from evidence. Selecting a control does not mark it complete.</p></aside>
      <section class="evidence-workbench" aria-label="Evidence workspace"><div class="evidence-heading"><div class="selected-control"><p class="eyebrow">${esc(selected?.title || 'Policy')} / CONTROL EVIDENCE</p><h2>${esc(control?.title || 'Select a control')}</h2><p>${esc(check.reason || 'Select a control to explore its evidence.')}</p></div><button class="evidence-focus" data-focus-evidence aria-pressed="${state.evidenceFocus}">${state.evidenceFocus?'Show checklist':'Expand evidence'}</button></div>${evidenceWorkspace(selected,control,check)}</section></div></div>`;
    bind();
    document.querySelectorAll('[data-disclosure]').forEach(details=>details.addEventListener('toggle',()=>{if(details.isConnected) state[details.dataset.disclosure]=details.open;}));
    const rerender = selector => {render(); document.querySelector(selector)?.focus();};
    document.querySelectorAll('[data-chain-action]').forEach(button=>button.addEventListener('click',()=>{
      state.openPolicies=button.dataset.chainAction==='expand'?new Set(children().map(p=>p.id)):new Set();
      rerender(`[data-chain-action="${button.dataset.chainAction}"]`);
    }));
    document.querySelector('[data-focus-evidence]').addEventListener('click',()=>{state.evidenceFocus=!state.evidenceFocus;rerender('[data-focus-evidence]');});
    document.querySelectorAll('[data-evidence-view]').forEach(button=>button.addEventListener('click',()=>{state.evidenceView=button.dataset.evidenceView;rerender(`[data-evidence-view="${state.evidenceView}"]`);}));
    document.querySelectorAll('[data-open-provenance]').forEach(button=>button.addEventListener('click',()=>{state.evidenceView='systems';rerender('[data-evidence-view="systems"]');}));
    document.querySelectorAll('[data-prov-node]').forEach(button=>button.addEventListener('click',()=>{state.provenanceNode=button.dataset.provNode;render();[...document.querySelectorAll('[data-prov-node]')].find(item=>item.dataset.provNode===state.provenanceNode)?.focus();}));
    document.querySelectorAll('[data-object]').forEach(button=>button.addEventListener('click',()=>{state.evidenceNode=button.dataset.object;state.evidenceLink=null;rerender(`[data-object="${state.evidenceNode}"]`);}));
    document.querySelectorAll('[data-link]').forEach(button=>button.addEventListener('click',()=>{state.evidenceLink=Number(button.dataset.link);rerender(`[data-link="${state.evidenceLink}"]`);}));
  }

  function network() {
    const result = evaluate();
    const selected = selectedPolicy();
    const control = selectedControl();
    const check = control ? checkFor(control.id,result) : {};
    const root = rootPolicy();
    const kids = children();
    const n = Math.max(kids.length, 1);
    const svgNodes = kids.map((p, i) => {
      const angle = -Math.PI / 2 + i * (Math.PI * 2 / n);
      const x = 50 + 36 * Math.cos(angle), y = 50 + 36 * Math.sin(angle);
      const s = controlsStatus(p,result);
      return {p,x,y,s};
    });
    const mapSvg = `<div class="map-plot"><svg class="policy-map" viewBox="0 0 100 100" role="img" aria-labelledby="map-title map-desc"><title id="map-title">Policy relationships</title><desc id="map-desc">The central ${esc(root.title)} links to ${kids.length} child policies. Select a policy node to inspect its controls.</desc><defs><marker id="arrow" markerWidth="4" markerHeight="4" refX="3" refY="2" orient="auto"><path d="M0 0L4 2L0 4" fill="#8aa4b7"></path></marker></defs>${svgNodes.map((node) => `<line class="map-edge" x1="50" y1="50" x2="${node.x}" y2="${node.y}" marker-end="url(#arrow)"></line>`).join('')}<circle class="root-halo" cx="50" cy="50" r="14"></circle></svg><div class="map-node-layer"><a href="index.html" class="map-root-node" aria-label="Open central policy overview"><span>CORE POLICY</span><b>${esc(root.title)}</b></a>${svgNodes.map((node) => `<button class="map-node ${node.p.id === selected?.id ? 'selected' : ''}" style="left:${node.x}%;top:${node.y}%" data-policy="${esc(node.p.id)}" aria-label="Select ${esc(node.p.title)}, ${node.s.met} of ${node.s.total} controls met"><span>${node.s.met === node.s.total ? '✓' : node.s.unknown ? '?' : '!'}</span><b>${esc(node.p.title)}</b><small>${node.s.met}/${node.s.total} met</small></button>`).join('')}</div></div>`;
    app.innerHTML = `<div class="page-shell network-page"><div class="page-heading"><div><p class="eyebrow">02 / POLICY NETWORK</p><h1>Follow the policy links</h1><p class="lede">Start at the central ship policy, then open a linked policy, control, and evidence record.</p></div>${selectScenario()}</div>${releaseBlock()}
      <div class="network-layout"><section class="network-canvas"><div class="network-toolbar"><div><p class="eyebrow">POLICY RELATIONSHIPS</p><h2>${esc(root.title)} <span>→ ${kids.length} child policies</span></h2></div><div class="map-legend"><span><i class="legend-core"></i>Central policy</span><span><i class="legend-child"></i>Child policy</span></div></div>${mapSvg}<div class="map-mobile-list"><a class="central-policy" href="index.html"><span class="central-icon">◆</span><span><b>${esc(root.title)}</b><small>Central release policy · overview</small></span></a>${kids.map((p) => `<button class="policy-row ${p.id === selected?.id ? 'selected' : ''}" data-policy="${esc(p.id)}"><span class="row-mark">●</span><span class="row-copy"><b>${esc(p.title)}</b><small>${esc(p.owner)}</small></span><span>${controlsStatus(p,result).met}/${controlsStatus(p,result).total}</span></button>`).join('')}</div><div class="map-caption"><span class="line-key"></span> required policy link <span class="caption-state">${decisionBadge(result.decision)} Shared evaluation</span></div></section>
      <aside class="network-inspector"><div class="inspector-header"><p class="eyebrow">SELECTED POLICY NODE</p><span class="node-index">${String(kids.findIndex((p)=>p.id===selected?.id)+1).padStart(2,'0')} / ${String(kids.length).padStart(2,'0')}</span></div>${crumb(selected,control)}<h2>${esc(selected?.title || 'Choose a policy')}</h2><p class="owner-line">Policy owner · ${esc(text(selected?.owner))}</p><p class="panel-description">${esc(text(selected?.description))}</p><div class="node-summary"><span>${badge(check.status)}</span><span>${controls(selected).length} controls</span><span>${evidenceFor(control?.id || '').length} evidence records</span></div><div class="network-controls"><h3>Controls in this policy</h3>${controls(selected).map((c) => {const ck=checkFor(c.id,result);return `<button class="network-control ${c.id===control?.id?'selected':''}" data-control="${esc(c.id)}" data-policy="${esc(selected?.id)}"><span>${esc(c.title)}</span>${badge(ck.status)}</button>`;}).join('') || '<p class="empty-state">No controls are listed.</p>'}</div>${control ? evidenceDetail(control,check) : ''}</aside></div>
      <div class="view-next"><span>Continue to another view</span><a href="board.html">Review readiness →</a><a href="console.html">Inspect decision trace →</a></div></div>`;
    bind();
  }

  function consoleView() {
    const result = evaluate();
    const selected = selectedPolicy();
    const control = selectedControl();
    const check = control ? checkFor(control.id,result) : {};
    const trace = (result.checks || []).map((c, index) => {
      const policy = data.policies.find((p) => p.id === c.policy_id);
      const ctrl = policy?.controls?.find((x) => x.id === c.control_id);
      return `<button class="trace-row ${c.control_id === control?.id ? 'current' : ''}" data-policy="${esc(c.policy_id)}" data-control="${esc(c.control_id)}"><span class="trace-number">${String(index + 1).padStart(2,'0')}</span><span class="trace-copy"><b>${esc(ctrl?.title || c.control_id)}</b><small>${esc(policy?.title || c.policy_id)} · ${esc(text(c.reason,'No result detail.'))}</small></span>${badge(c.status)}</button>`;
    }).join('');
    const decisionClass = result.decision === 'allow' ? 'allow' : result.decision === 'deny' ? 'deny' : 'indeterminate';
    app.innerHTML = `<div class="page-shell console-page"><div class="page-heading"><div><p class="eyebrow">03 / SIMULATED EVALUATION</p><h1>Inspect the decision trace</h1><p class="lede">Change the sample scenario. The shared evaluator recalculates every control and the release decision.</p></div>${selectScenario()}</div>${releaseBlock()}
      <section class="console-result ${decisionClass}"><div class="result-pulse"><span></span></div><div class="result-main"><small>SIMULATED EVALUATION RESULT</small><h2>${decisionName(result.decision)}</h2><p>${esc(resultReason(result) || 'All required controls have matching evidence.')}</p></div><div class="result-attrs"><span><small>SCENARIO</small><b>${esc(scenarios.find(([v])=>v===state.scenario)?.[1] || state.scenario)}</b></span><span><small>POLICIES CHECKED</small><b>${children().length}</b></span><span><small>CONTROLS CHECKED</small><b>${(result.checks || []).length}</b></span></div></section>
      <div class="console-grid"><section class="trace-panel"><div class="section-heading"><div><p class="eyebrow">DECISION TRACE</p><h2>Required control checks</h2></div><span class="count-label">ORDERED CHECKS</span></div><div class="trace-list">${trace || '<p class="empty-state">The evaluator returned no control checks.</p>'}</div><div class="trace-rule"><b>Decision order</b><span>deny → indeterminate → allow</span><small>Any failed required control denies the release. Unknown evidence returns an indeterminate result.</small></div></section>
       <aside class="trace-detail"><div class="trace-detail-head"><p class="eyebrow">CHECK DETAIL</p>${crumb(selected,control)}</div><h2>${esc(control?.title || 'Select a control check')}</h2><p class="detail-reason">${esc(text(check.reason, 'Select a row in the decision trace to see its evidence and result.'))}</p>${control ? `<div class="decision-step"><span class="step-dot ${statusClass(check.status)}">${check.status === 'met' ? '✓' : check.status === 'not_met' ? '!' : '?'}</span><div><b>${statusName(check.status)}</b><small>Required control check</small></div></div>${evidenceDetail(control,check)}` : ''}</aside></div>
       <div class="scenario-explainer"><b>Scenario behavior</b><span>${state.scenario === 'ready' ? 'Matching, current evidence supports an allow result.' : state.scenario === 'blocked' ? 'A control is not met. The evaluator denies the release.' : state.scenario === 'missing' ? 'A required evidence record is absent. The evaluator returns indeterminate.' : state.scenario === 'expired' ? 'The evidence is past its expiry time. The evaluator returns indeterminate.' : 'The evidence names another artifact. The evaluator returns indeterminate.'}</span></div>
       <div class="view-next"><span>Continue to another view</span><a href="board.html">Review readiness →</a><a href="network.html">Map policy links →</a></div></div>`;
    bind();
  }

  function render() {
    document.body.dataset.mode = mode;
    try { ({ home, board, network, console: consoleView })[mode](); }
    catch (error) { app.innerHTML = `<section class="load-error"><h1>Could not render this view</h1><p>${esc(error.message)}</p><a href="index.html">Return to overview</a></section>`; }
  }

  fetch('data.json').then((response) => { if (!response.ok) throw new Error(`Could not load data.json (${response.status}).`); return response.json(); }).then((loaded) => {
    data = loaded.model || loaded;
    if (data.schema !== 'ipe.product-ship-demo.v1') throw new Error('The sample data uses an unsupported schema.');
    if (!Array.isArray(data.policies) || !Array.isArray(data.evidence) || !data.release) throw new Error('The sample data is incomplete.');
    if (!scenarios.some(([value]) => value === state.scenario)) state.scenario = 'ready';
    state.policyId = children().some((p) => p.id === state.policyId) ? state.policyId : (children()[0]?.id || '');
    state.controlId = controls(children().find((p) => p.id === state.policyId)).some((c) => c.id === state.controlId) ? state.controlId : '';
    state.openPolicies.add(state.policyId);
    render();
  }).catch((error) => { app.innerHTML = `<section class="load-error"><p class="eyebrow">SAMPLE DATA UNAVAILABLE</p><h1>Could not load the product ship demo</h1><p>${esc(error.message)} Start the local server with <code>python3 tools/product-ship-mocks/serve.py</code> and make sure the sample runner created <code>tools/product-ship-mocks/data.json</code>.</p></section>`; });
})();
