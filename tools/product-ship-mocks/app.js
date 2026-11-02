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
  let data;

  const esc = (value) => String(value ?? '').replace(/[&<>"']/g, (c) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const text = (value, fallback = 'Not provided') => value == null || value === '' ? fallback : String(value);
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
  const setPolicy = (id) => { state.policyId = id; state.controlId = ''; state.evidenceIndex = 0; render(); };
  const bind = () => {
    document.querySelectorAll('.central-icon,.central-arrow,.row-mark,.control-symbol,.trace-number,.map-node span').forEach((element) => element.setAttribute('aria-hidden', 'true'));
    const query = new URLSearchParams();
    if (state.scenario !== 'ready') query.set('scenario', state.scenario);
    if (state.policyId) query.set('policy', state.policyId);
    if (state.controlId) query.set('control', state.controlId);
    document.querySelectorAll('a[href]').forEach((link) => {
      const target = link.getAttribute('href').split('?')[0];
      if (!['index.html','board.html','network.html','console.html'].includes(target)) return;
      link.href = query.size ? `${target}?${query.toString()}` : target;
    });
    document.querySelector('#scenario')?.addEventListener('change', (event) => { state.scenario = event.target.value; render(); });
    document.querySelectorAll('[data-policy]:not([data-control])').forEach((button) => button.addEventListener('click', () => setPolicy(button.dataset.policy)));
    document.querySelectorAll('[data-control]').forEach((button) => button.addEventListener('click', () => { state.policyId = button.dataset.policy || state.policyId; state.controlId = button.dataset.control; state.evidenceIndex = 0; render(); }));
    document.querySelectorAll('[data-evidence-index]').forEach((button) => button.addEventListener('click', () => { state.evidenceIndex = Number(button.dataset.evidenceIndex); render(); }));
  };

  function home() {
    const result = evaluate();
    const counts = children().reduce((sum, p) => { const s = controlsStatus(p, result); sum.total += s.total; sum.met += s.met; sum.unknown += s.unknown; sum.notMet += s.notMet; return sum; }, {total:0, met:0,unknown:0,notMet:0});
    app.innerHTML = `<section class="home-hero"><div class="hero-copy"><p class="eyebrow">PRODUCT SHIP QUALIFICATION</p><h1>One release.<br><span>Every policy in view.</span></h1><p>Review the central ship policy and its linked controls for accessibility, security, open-source legal, and enterprise requirements.</p><div class="hero-actions"><a class="primary-link" href="board.html">Review release readiness <span aria-hidden="true">→</span></a><span class="simulation-note">SIMULATED ENFORCEMENT · FAIL CLOSED</span></div></div><div class="hero-stamp"><span>SHIP</span><b>POLICY</b><small>RELEASE GATE</small><div class="stamp-lines">${Array.from({length:5},() => '<i></i>').join('')}</div></div></section>
      ${releaseBlock()}<section class="overview-grid"><div class="overview-intro"><p class="eyebrow">CHOOSE A VIEW</p><h2>Explore the release gate</h2><p>Each view uses the same release data and simulated evaluator. Select a policy to inspect its controls and evidence.</p></div>
       <a class="view-card card-board" href="board.html"><span class="card-number">01 / READINESS</span><div class="card-visual"><div class="mini-gate"><i></i><i></i><i></i><b>${counts.met}/${counts.total}</b></div></div><h3>Release gate board <span>→</span></h3><p>See blockers, missing evidence, and control status at a glance.</p></a>
       <a class="view-card card-network" href="network.html"><span class="card-number">02 / RELATIONSHIPS</span><div class="card-visual network-mini"><b>${esc(rootPolicy().title)}</b><i></i><i></i><i></i><small>${children().length} linked policies</small></div><h3>Policy network <span>→</span></h3><p>Move from the central policy into each child policy and control.</p></a>
       <a class="view-card card-console" href="console.html"><span class="card-number">03 / DECISION TRACE</span><div class="card-visual console-mini"><span>evaluate()</span><b>${decisionName(result.decision)}</b><i></i><i></i></div><h3>Enforcement console <span>→</span></h3><p>Change a scenario and inspect the shared evaluation trace.</p></a>
      </section><section class="home-summary"><div><small>SIMULATED RESULT</small>${decisionBadge(result.decision)}</div><p>${counts.met} met <span>·</span> ${counts.notMet} not met <span>·</span> ${counts.unknown} unknown</p>${selectScenario()}</section>`;
    bind();
  }

  function board() {
    const result = evaluate();
    const selected = selectedPolicy();
    const control = selectedControl();
    const check = control ? checkFor(control.id, result) : {};
    const status = children().reduce((sum, policy) => {
      const current = controlsStatus(policy, result);
      sum.met += current.met;
      sum.total += current.total;
      sum.unknown += current.unknown;
      sum.notMet += current.notMet;
      return sum;
    }, { met: 0, total: 0, unknown: 0, notMet: 0 });
    const blockers = (result.checks || []).filter((c) => c.status !== 'met');
    app.innerHTML = `<div class="page-shell board-page"><div class="page-heading"><div><p class="eyebrow">01 / RELEASE GATE BOARD</p><h1>Readiness at a glance</h1><p class="lede">A clear view of what can ship, what blocks release, and what needs evidence.</p></div>${selectScenario()}</div>${releaseBlock()}
      <section class="readiness-strip"><div class="readiness-result"><small>SIMULATED RELEASE DECISION</small>${decisionBadge(result.decision)}<p>${esc(resultReason(result) || 'All required controls have matching evidence.')}</p></div><div class="readiness-metric"><strong>${status.met}<small>/${status.total}</small></strong><span>Controls met</span></div><div class="readiness-metric warn"><strong>${blockers.length}</strong><span>Items needing action</span></div><div class="readiness-progress"><div class="progress-meta"><span>Evidence coverage</span><b>${status.total ? Math.round(100 * status.met / status.total) : 0}%</b></div><div class="progress-track" role="progressbar" aria-label="Required controls met" aria-valuemin="0" aria-valuemax="${status.total}" aria-valuenow="${status.met}"><i style="width:${status.total ? 100 * status.met / status.total : 0}%"></i></div><small>${status.met} of ${status.total} required controls met</small></div></section>
      <div class="board-layout"><section class="policy-rail"><div class="section-heading"><div><p class="eyebrow">POLICY CHAIN</p><h2>Ship requirements</h2></div><span class="count-label">${children().length} POLICIES</span></div><a class="central-policy" href="index.html"><span class="central-icon">◆</span><span><b>${esc(rootPolicy().title)}</b><small>Central release policy · overview</small></span><span class="central-arrow">→</span></a><div class="policy-rows">${children().map((p) => { const s=controlsStatus(p,result); return `<button class="policy-row ${p.id === selected?.id ? 'selected' : ''}" data-policy="${esc(p.id)}"><span class="row-mark">${p.id === selected?.id ? '●' : '○'}</span><span class="row-copy"><b>${esc(p.title)}</b><small>${esc(text(p.owner,'Policy owner'))}</small></span><span class="row-count">${s.met}/${s.total}</span><span class="row-arrow">›</span></button>`; }).join('')}</div></section>
      <section class="control-panel"><div class="section-heading"><div><p class="eyebrow">${esc(selected?.title || 'Policy')}</p><h2>${esc(text(selected?.owner,'Policy owner'))}</h2></div><a class="text-link" href="network.html">View policy map ↗</a></div><p class="panel-description">${esc(text(selected?.description))}</p><div class="control-list">${controls(selected).map((c) => { const ck=checkFor(c.id,result); return `<button class="control-row ${c.id === control?.id ? 'chosen' : ''}" data-control="${esc(c.id)}" data-policy="${esc(selected.id)}"><span class="control-symbol">${ck.status === 'met' ? '✓' : ck.status === 'not_met' ? '!' : '?'}</span><span><b>${esc(c.title)}</b><small>${esc(c.description)}</small></span>${badge(ck.status)}</button>`; }).join('') || '<p class="empty-state">No controls are listed for this policy.</p>'}</div></section>
      <aside class="evidence-panel">${crumb(selected, control)}${control ? `<div class="selected-control"><p class="eyebrow">SELECTED CONTROL</p><h2>${esc(control.title)}</h2><p>${esc(text(control.description))}</p></div>${evidenceDetail(control,check)}` : '<p class="empty-state">Select a control to inspect its evidence.</p>'}</aside></div>
      <div class="view-next"><span>Continue to another view</span><a href="network.html">Map policy links →</a><a href="console.html">Inspect decision trace →</a></div></div>`;
    bind();
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
    render();
  }).catch((error) => { app.innerHTML = `<section class="load-error"><p class="eyebrow">SAMPLE DATA UNAVAILABLE</p><h1>Could not load the product ship demo</h1><p>${esc(error.message)} Start the local server with <code>python3 tools/product-ship-mocks/serve.py</code> and make sure the sample runner created <code>tools/product-ship-mocks/data.json</code>.</p></section>`; });
})();
