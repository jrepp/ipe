'use strict';
const $ = s => document.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const pretty = value => esc(JSON.stringify(value, null, 2));
const url = value => {try {const u = new URL(value);return u.protocol === 'https:' && u.hostname === 'github.com' ? esc(u.href) : '#';} catch {return '#';}};
let report, selected = 0;
function rows(object) {return '<table><tbody>' + Object.entries(object || {}).map(([key,value]) => `<tr><th>${esc(key)}</th><td><pre>${pretty(value)}</pre></td></tr>`).join('') + '</tbody></table>';}
function sources() {
  const query = $('#search').value.toLowerCase();
  const documents = report.documents.filter(d => `${d.source.repository} ${d.source.path} ${d.projection.model.name || ''}`.toLowerCase().includes(query));
  $('#sources').innerHTML = documents.map(d => `<button data-document="${d.id}" class="${d.id===selected?'active':''}" aria-pressed="${d.id===selected}"><small>${esc(d.kind)} · ${esc(d.source.commit.slice(0,8))}</small><strong>${esc(d.source.repository)}</strong><small>${esc(d.source.path)}</small></button>`).join('') || '<p class="empty">No sources match.</p>';
}
function detail() {
  const d = report.documents.find(d => d.id === selected);
  if (!d) return;
  const p = d.projection, m = p.model;
  const children = report.edges.filter(e => e.from === d.id);
  const callers = report.edges.filter(e => e.to === d.id);
  const bindings = callers.map(e => `<details open><summary>Inputs at call site: ${esc(report.documents[e.from].source.repository)} <code>${esc(e.pointer)}</code></summary><p>These bindings describe declarations. Required input flags do not prove runtime validation.</p><table><thead><tr><th>Input</th><th>Value source</th><th>Value</th></tr></thead><tbody>${(e.bindings?.inputs||[]).map(b=>`<tr><th>${esc(b.input)}</th><td>${esc(b.origin)}</td><td><pre>${pretty(b.value)}</pre></td></tr>`).join('')}</tbody></table>${e.bindings?.undeclared_caller_inputs?.length?`<p>Undeclared inputs: ${esc(e.bindings.undeclared_caller_inputs.join(', '))}</p>`:''}</details>`).join('');
  const jobs = (m.jobs || []).map(j => `<details open><summary><strong>${esc(j.name || j.id)}</strong> <code>${esc(j.id)}</code></summary>${rows(Object.fromEntries(Object.entries(j).filter(([k]) => !['steps','id','pointer'].includes(k))))}<h3>Steps</h3>${(j.steps||[]).map((s,i) => `<details><summary>${i+1}. ${esc(s.name || s.uses || 'Run command')}</summary>${rows(s)}</details>`).join('')}</details>`).join('');
  $('#detail').innerHTML = `<span class="tag">Declared · partial coverage</span><span class="tag">${d.ref_was_pinned?'Commit-pinned reference':'Moving reference resolved'}</span><h2>${esc(m.name || d.source.path)}</h2><p class="source-meta"><a href="${url(d.source.url)}" target="_blank" rel="noopener noreferrer">Open exact source ↗</a><br>Requested ${esc(d.requested_ref)} → <code>${esc(d.source.commit)}</code><br><code>${esc(d.source.digest)}</code></p><h3>${d.kind==='workflow'?'Workflow declarations':'Action interface and runtime'}</h3>${rows(Object.fromEntries(Object.entries(m).filter(([k]) => k!=='jobs')))}${jobs}${bindings}<h3>Referenced actions and workflows</h3>${children.map(e => {const child=report.documents.find(d=>d.id===e.to);return `<button class="dependency" data-document="${e.to}">${esc(child.source.repository)} · ${esc(child.source.path)}<br><small>${esc(e.pointer)}</small></button>`;}).join('') || '<p>No resolved child sources.</p>'}<h3>Unresolved expressions (${p.expressions.length})</h3>${p.expressions.map(e => `<details><summary><code>${esc(e.pointer)}</code></summary><pre>${esc(e.expression)}</pre>${d.locations?.[e.pointer]?`<a href="${url(d.source.url+'#L'+d.locations[e.pointer].line)}" target="_blank" rel="noopener noreferrer">Source line ${d.locations[e.pointer].line} ↗</a>`:''}<p>Requires GitHub execution context; not evaluated by this capture.</p></details>`).join('')}<h3>Implementation evidence</h3>${d.implementation.map(e => e.status==='captured'?`<details><summary>${esc(e.source.path)} · ${esc(e.role)}</summary><p><a href="${url(e.source.url)}" target="_blank" rel="noopener noreferrer">Pinned source ↗</a> · ${e.source.bytes} bytes · ${e.excerpt_truncated?'Excerpt only':'Full text'}</p>${(e.clues||[]).length?'<h4>Lexical clues — not verified effects</h4>'+e.clues.map(c=>`<p><a href="${url(e.source.url+'#L'+c.line)}" target="_blank" rel="noopener noreferrer">Line ${c.line}</a> <code>${esc(c.text)}</code></p>`).join(''):''}<pre>${esc(e.excerpt)}</pre></details>`:`<p class="gap"><code>${esc(e.code)}</code>${esc(e.path || '')}</p>`).join('') || '<p>Implementation evidence is collected for actions.</p>'}<details><summary>Original YAML</summary><pre>${esc(d.source_text)}</pre></details><details><summary>Agent projection JSON</summary><pre>${pretty(p)}</pre></details>`;
  sources();
}
function show(data) {
  if(data.schema!=='ipe.github-contract.v1' || !Array.isArray(data.documents) || !data.documents.length) throw new Error('Unsupported or empty capture');
  report=data;
  if (URL.createObjectURL) $('#download-json').href=URL.createObjectURL(new Blob([JSON.stringify(data,null,2)],{type:'application/json'}));
  const expr=data.documents.reduce((n,d)=>n+d.projection.expressions.length,0);
  const clues=data.documents.reduce((n,d)=>n+d.implementation.reduce((a,e)=>a+(e.clues||[]).length,0),0);
  $('#summary').innerHTML=`<p class="source-meta">${esc(data.scope.repository)} · ${esc(data.scope.workflow)}<br>Observed ${esc(new Date(data.observed_at_unix*1000).toISOString())}</p><div class="metrics">${[[data.documents.length,'captured definitions'],[data.edges.length,'dependency links'],[expr,'unresolved expressions'],[clues,'implementation clues']].map(([n,s])=>`<div><strong>${n}</strong><span>${s}</span></div>`).join('')}</div>`;
  const root=data.documents[0];
  $('#story-detail').innerHTML=`<strong>Start with ${esc(root.projection.model.name || root.source.path)}.</strong> This capture follows ${(root.projection.model.jobs||[]).length} declared jobs into ${data.documents.filter(d=>d.kind==='action').length} action definitions. Every source link names an exact commit. ${expr} expressions remain unresolved. Open an action below to compare its advertised interface with its implementation clues.`;
  const groups=new Map();
  for(const d of data.documents){const items=groups.get(d.source.repository)||[];items.push(d);groups.set(d.source.repository,items);}
  const revisions=[...groups].filter(([,items])=>new Set(items.map(d=>d.source.commit)).size>1);
  const defaults=data.edges.reduce((n,e)=>n+(e.bindings?.inputs||[]).filter(b=>b.origin==='declared_default').length,0);
  $('#story-detail').innerHTML+=`<h3>What this evidence changes in our model</h3><p>${defaults} input bindings come from declared defaults. An omitted caller value therefore cannot be modeled as an absent requirement or effect.</p>${revisions.map(([repo,items])=>`<p><strong>${esc(repo)}</strong> appears at ${new Set(items.map(d=>d.source.commit)).size} resolved commits. Definition identity needs repository, commit, and path. Invocation identity also needs the caller and source location.</p>${items.map(d=>`<button class="dependency" data-document="${d.id}">Inspect ${esc(d.source.repository)} @ ${esc(d.source.commit.slice(0,12))}</button>`).join('')}`).join('')}<p>Next proof: join a real run, artifact, and deployment to these declarations. Keep the run outcome separate from the definition and from source-code clues.</p>`;
  const gaps=[...data.gaps,...data.documents.flatMap(d=>[...d.projection.gaps.map(g=>({...g,repository:d.source.repository})),...d.implementation.filter(e=>e.status!=='captured').map(e=>({...e,repository:d.source.repository}))])];
  $('#gap-list').innerHTML=gaps.map(g=>`<div class="gap"><code>${esc(g.code)}</code>${esc(g.repository || '')} ${esc(g.pointer || g.path || '')}</div>`).join('')+'<ul>'+data.limitations.map(l=>`<li>${esc(l)}</li>`).join('')+'</ul>';
  detail();
}
document.addEventListener('click',e=>{const b=e.target.closest('[data-document]');if(b){selected=Number(b.dataset.document);detail();}});
$('#search').addEventListener('input',sources);
(async()=>{try{const embedded=document.getElementById('capture-data');let data;if(embedded)data=JSON.parse(embedded.textContent);else{const response=await fetch('capture.json');if(!response.ok)throw new Error('Capture could not be loaded');data=await response.json();}show(data);}catch{$('#summary').innerHTML='<p class="error">Capture could not be loaded. Serve the report directory locally or open the standalone report.</p>';}})();
