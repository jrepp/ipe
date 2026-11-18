/* Focused domain inspection. A provider outcome does not prove requirement fulfillment. */
(() => {
  'use strict';
  const sessions = new WeakMap();
  const text = value => String(value ?? 'Unknown');
  const words = value => text(value).replaceAll('_', ' ');
  const entity = node => node?.detail?.record?.model?.entity;
  const actorText = value => ['unknown', 'unobserved', 'intended_execution_actor_unobserved'].includes(value) || !value ? 'Execution actor not observed' : text(value);
  const applicabilityText = value => value?.expression || (value?.default_behavior === 'provider_default_dependency_status' ? 'Provider dependency rules; result unknown' : 'Applicability not established');
  const specificationText = kind => ({opaque:'Uninterpreted condition',permission:'Requested permission',environment:'Environment reference',dependency:'Operation dependency',fact:'Evidence condition'}[kind] || words(kind));
  const date = value => {
    const parsed = new Date(typeof value === 'number' ? value * 1000 : NaN);
    return Number.isFinite(parsed.getTime()) ? parsed.toISOString().replace('T', ' ').replace('.000Z', ' UTC') : 'Unknown';
  };
  const short = (value, length = 34) => { const s = text(value); return s.length > length ? s.slice(0, length - 1) + '…' : s; };
  let sequence = 0;

  function el(doc, tag, className, value) {
    const node = doc.createElement(tag);
    if (className) node.className = className;
    if (value !== undefined) node.textContent = value;
    return node;
  }
  function json(value) {
    const result = JSON.stringify(value ?? {}, null, 2);
    return result.length > 32768 ? result.slice(0, 32768) + '\n… Display limited to 32 KiB. Inspect the full run below.' : result;
  }
  function create(container) {
    const doc = container.ownerDocument;
    const root = el(doc, 'section', 'ipe-trace ipe-domain');
    root.setAttribute('aria-label', 'Operation and execution evidence');
    const heading = el(doc, 'h3', 'ipe-trace-heading');
    const intro = el(doc, 'p', 'ipe-domain-intro', 'Inspect one declared operation and its observed execution. Captured declarations do not establish a policy, contract, or operating control.');
    const selectors = el(doc, 'div', 'ipe-domain-selectors');
    const operationLabel = el(doc, 'label', '', 'Operation');
    const operationSelect = el(doc, 'select', 'ipe-domain-operation');
    operationSelect.setAttribute('aria-label', 'Declared operation');
    operationLabel.append(operationSelect);
    const executionLabel = el(doc, 'label', '', 'Observed execution');
    const executionSelect = el(doc, 'select', 'ipe-domain-execution');
    executionSelect.setAttribute('aria-label', 'Observed execution');
    executionLabel.append(executionSelect);
    selectors.append(operationLabel, executionLabel);
    const state = el(doc, 'p', 'ipe-trace-state');
    const cards = el(doc, 'div', 'ipe-domain-cards');
    const linkCard = el(doc, 'div', 'ipe-domain-link-status');
    linkCard.setAttribute('role', 'status');
    const outcomeCard = el(doc, 'div', 'ipe-domain-outcome');
    const fulfillmentCard = el(doc, 'div', 'ipe-domain-fulfillment');
    cards.append(linkCard, outcomeCard, fulfillmentCard);
    const overview = el(doc, 'div', 'ipe-domain-overview');
    const requirements = el(doc, 'details', 'ipe-domain-requirements');
    requirements.open = true;
    const requirementsTitle = el(doc, 'summary');
    const requirementsList = el(doc, 'ul');
    requirements.append(requirementsTitle, requirementsList);
    const viewport = el(doc, 'div', 'ipe-trace-viewport');
    viewport.setAttribute('role', 'region');
    viewport.setAttribute('aria-label', 'Selected operation, observed execution, and facts');
    viewport.tabIndex = 0;
    const legend = el(doc, 'p', 'ipe-domain-legend', 'Dashed: candidate operation link. Solid: exact execution identity.');
    const empty = el(doc, 'p', 'ipe-domain-empty');
    const pagination = el(doc, 'div', 'ipe-domain-pagination');
    const previous = el(doc, 'button', 'ipe-domain-previous', 'Previous facts');
    const next = el(doc, 'button', 'ipe-domain-next', 'Next facts');
    previous.type = next.type = 'button';
    const pageLabel = el(doc, 'span', 'ipe-domain-page');
    pagination.append(previous, pageLabel, next);
    const details = el(doc, 'section', 'ipe-trace-details');
    details.setAttribute('aria-label', 'Selected domain record');
    const detailHeading = el(doc, 'h4', '', 'Select a record');
    const detailSummary = el(doc, 'p');
    detailSummary.setAttribute('role', 'status');
    const detailFields = el(doc, 'dl', 'ipe-domain-detail-fields');
    const rawDetails = el(doc, 'details', 'ipe-domain-raw');
    const rawTitle = el(doc, 'summary', '', 'Raw retained record');
    const detailBody = el(doc, 'pre');
    detailBody.hidden = true;
    rawDetails.hidden = true;
    rawDetails.append(rawTitle, detailBody);
    details.append(detailHeading, detailSummary, detailFields, rawDetails);
    const collection = el(doc, 'details', 'ipe-domain-collection');
    const collectionTitle = el(doc, 'summary', '', 'Collection evidence and source records');
    const collectionDescription = el(doc, 'p');
    const issueList = el(doc, 'ul', 'ipe-trace-issues');
    const collectionBody = el(doc, 'pre');
    collection.append(collectionTitle, collectionDescription, issueList, collectionBody);
    root.append(heading, intro, selectors, state, cards, overview, requirements, legend, viewport, empty, pagination, details, collection);
    container.replaceChildren(root);
    const session = {root,heading,operationSelect,executionSelect,state,linkCard,outcomeCard,fulfillmentCard,overview,requirements,requirementsTitle,requirementsList,viewport,empty,previous,next,pageLabel,pagination,detailHeading,detailSummary,detailFields,rawDetails,detailBody,collection,collectionDescription,collectionBody,issueList,selection:{},selected:null,marker:`ipe-domain-arrow-${++sequence}`};
    sessions.set(container, session);
    operationSelect.onchange = () => { session.selection = {operationId:operationSelect.value}; session.selected = null; draw(session); };
    executionSelect.onchange = () => { session.selection.executionId = executionSelect.value; session.selection.factPage = 0; session.selected = null; draw(session); };
    previous.onclick = () => { session.selection.factPage = Math.max(0, session.focus.page - 1); draw(session); };
    next.onclick = () => { session.selection.factPage = session.focus.page + 1; draw(session); };
    return session;
  }
  function options(select, values, selected, empty) {
    const list = values.length ? values : [{id:'',label:empty}];
    globalThis.d3.select(select).selectAll('option').data(list, item => item.id).join('option').attr('value', item => item.id).text(item => item.label);
    select.disabled = !values.length;
    select.value = selected ?? list[0].id;
  }
  function card(container, title, value, note) {
    const doc = container.ownerDocument;
    container.replaceChildren(el(doc,'span','ipe-domain-card-title',title),el(doc,'strong','',value),el(doc,'p','',note));
  }
  function selectNode(session, node) {
    session.selected = node?.id ?? null;
    session.detailHeading.textContent = node ? text(node.label) : 'Select a record';
    session.detailSummary.textContent = node ? `${node.focusLane === 'operation' ? 'Operation declaration' : node.focusLane === 'execution' ? 'Provider execution result' : 'Fact emission'} · ${words(node.status)} · ${Number.isInteger(node.step) ? `Step ${node.step}` : 'Exact step not recorded'}` : 'Select an operation, execution, or fact to inspect its retained data.';
    session.detailBody.hidden = !node;
    session.rawDetails.hidden = !node;
    const e = entity(node), record = node?.detail?.record, fields = [];
    if (node?.focusLane === 'fact') {
      fields.push(['Predicate', e?.predicate || 'Not retained'], ['Value', e && 'value' in e ? (typeof e.value === 'string' ? e.value : JSON.stringify(e.value)) : 'Not retained'], ['Reported authority', e?.authority || 'Not retained'], ['Observed', date(record?.observed_at)], ['Expires', date(record?.expires_at)]);
    } else if (node?.focusLane === 'execution') {
      const identity = e?.execution;
      fields.push(['Provider result', words(e?.conclusion || e?.status)], ['Execution', identity ? `${identity.resource} · run ${identity.run} · attempt ${identity.attempt}${identity.job ? ` · job ${identity.job}` : ''}${identity.step ? ` · step ${identity.step}` : ''}` : 'Not retained'], ['Started', e?.started_at || 'Unknown'], ['Completed', e?.completed_at || 'Unknown']);
    } else if (node?.focusLane === 'operation') {
      fields.push(['Actor', actorText(node.actor)], ['Applicability', applicabilityText(node.applicability)], ['Declared enforcer', node.declaredEnforcer || 'Unknown']);
    }
    const source = e?.source || e?.definition;
    if (source) fields.push(['Source', `${text(source.definition?.resource)} / ${text(source.definition?.path)}`], ['Source revision', text(source.revision)]);
    session.detailFields.replaceChildren(...fields.flatMap(([name,value]) => [el(session.root.ownerDocument,'dt','',name),el(session.root.ownerDocument,'dd','',short(value,1200))]));
    session.detailBody.textContent = node ? json(node.detail) : '';
    session.nodeSelection?.attr('aria-pressed', item => String(item.id === session.selected)).classed('is-selected', item => item.id === session.selected);
    session.edgeSelection?.classed('is-selected', edge => edge.source === session.selected || edge.target === session.selected);
  }
  function draw(session) {
    const focus = globalThis.IpeTraceModel.buildDomainFocus(session.run, session.selection);
    session.focus = focus;
    session.selection = {...focus.selection};
    const {operation,execution} = focus;
    const opOptions = focus.operations.map(node => ({id:node.id,label:text(node.label)}));
    if (!opOptions.length || focus.unmapped.executions || focus.unmapped.facts) opOptions.push({id:'__unmapped__',label:`Unmapped executions and facts (${focus.unmapped.executions} / ${focus.unmapped.facts})`});
    options(session.operationSelect, opOptions, session.selection.operationId, 'No operation declarations');
    const executionOptions = focus.executions.map(node => {
      const e=entity(node);
      return {id:node.id,label:`${text(node.label)} · ${text(e?.conclusion || e?.status || node.status)}${e?.execution?.attempt ? ` · attempt ${e.execution.attempt}` : ''}`};
    });
    if (focus.mode === 'unmapped' && focus.unmapped.facts) executionOptions.push({id:'__unlinked_facts__',label:'Facts without a retained execution link'});
    options(session.executionSelect, executionOptions, session.selection.executionId, 'No linked execution');
    const candidate = focus.edges.some(edge => edge.source === operation?.id && edge.target === execution?.id && edge.status === 'candidate');
    card(session.linkCard, 'Operation-to-execution link', candidate ? 'Candidate' : operation && execution ? 'Exact retained link' : operation ? 'No linked execution' : 'Unmapped', candidate ? 'This execution may belong to the operation. The relationship is not proven.' : operation && execution ? 'The retained reference identifies this execution.' : operation ? 'No provider execution can be selected for this declaration.' : 'These records have no selected operation relationship.');
    const result = entity(execution);
    card(session.outcomeCard, 'Observed execution result', execution ? words(result?.conclusion || result?.status || execution.status) : 'Not observed', execution ? 'Provider-reported result for this execution only.' : 'No execution result is available for this selection.');
    card(session.fulfillmentCard, 'Requirement fulfillment', 'Unknown', 'Execution success and collected facts do not establish that every requirement was met.');
    session.linkCard.classList.toggle('is-uncertain', candidate || !operation || !execution);
    const doc=session.root.ownerDocument;
    const metadata=operation ? [['Actor',actorText(operation.actor)],['Applicability',applicabilityText(operation.applicability)],['Declared enforcer',operation.declaredEnforcer]] : [];
    const dl=el(doc,'dl');
    for(const [name,value] of metadata) dl.append(el(doc,'dt','',name),el(doc,'dd','',text(value)));
    session.overview.replaceChildren(...(operation?[dl]:[el(doc,'p','','No operation declaration is available for these records. Inspect them without inferring a governing rule.')]));
    const requirements=operation?.requirements || [];
    session.requirements.hidden = !operation;
    session.requirementsTitle.textContent=`${requirements.length} declared requirements · fulfillment unknown`;
    session.requirementsList.replaceChildren(...requirements.map(item=>{
      const row=el(doc,'li');row.append(el(doc,'strong','',text(item.description)),el(doc,'span','ipe-domain-spec',`${specificationText(item.specification?.kind)} · evidence needed`));return row;
    }));
    if (!requirements.length) session.requirementsList.append(el(doc,'li','','No requirement items were retained. This does not establish that no requirements apply.'));
    session.state.textContent=session.run?.status==='succeeded'?'Records were committed to the local host. Facts do not prove requirement fulfillment.':`Run ${words(session.run?.status)}. Retained emissions are staged or discarded; they are not committed records.`;
    if(focus.legacy)session.state.textContent+=' Legacy run: exact emission steps were not retained.';
    if(focus.omitted?.emissions)session.state.textContent+=` ${focus.omitted.emissions} native emissions were not retained.`;
    const nodes=[...(operation?[{...operation,focusLane:'operation'}]:[]),...(execution?[{...execution,focusLane:'execution'}]:[]),...focus.facts.map(node=>({...node,focusLane:'fact'}))];
    const positions=new Map(nodes.map(node=>[node.id,{x:node.focusLane==='operation'?20:node.focusLane==='execution'?360:700,y:node.focusLane==='fact'?58+focus.facts.findIndex(f=>f.id===node.id)*88:58}]));
    const height=Math.max(160,70+focus.facts.length*88),nodeWidth=280,nodeHeight=68;
    const svg=globalThis.d3.select(session.viewport).selectAll('svg').data([null]).join('svg').attr('class','ipe-domain-svg').attr('width',1000).attr('height',height).attr('viewBox',`0 0 1000 ${height}`).attr('role','group').attr('aria-label','Operation, observed execution, and selected facts');
    svg.selectAll('.ipe-domain-lane').data([{x:20,label:'Operation'},{x:360,label:'Observed execution'},{x:700,label:'Facts'}]).join('text').attr('class','ipe-domain-lane').attr('x',lane=>lane.x).attr('y',29).text(lane=>lane.label);
    const marker=svg.selectAll('defs').data([null]).join('defs').selectAll('marker').data([null]).join('marker').attr('id',session.marker).attr('viewBox','0 -4 8 8').attr('refX',8).attr('markerWidth',7).attr('markerHeight',7).attr('orient','auto');
    marker.selectAll('path').data([null]).join('path').attr('d','M0,-4L8,0L0,4').attr('fill','#718c81');
    const edges=focus.edges.filter(edge=>positions.has(edge.source)&&positions.has(edge.target));
    session.edgeSelection=svg.selectAll('.ipe-domain-edges').data([null]).join('g').attr('class','ipe-domain-edges').selectAll('path').data(edges,edge=>edge.id).join('path').attr('class',edge=>`ipe-trace-edge${edge.status==='candidate'?' is-candidate':''}`).attr('marker-end',`url(#${session.marker})`).attr('d',edge=>{
      const a=positions.get(edge.source),b=positions.get(edge.target),x=a.x+nodeWidth,m=(x+b.x)/2;
      return `M${x},${a.y+nodeHeight/2} C${m},${a.y+nodeHeight/2} ${m},${b.y+nodeHeight/2} ${b.x},${b.y+nodeHeight/2}`;
    });
    session.edgeSelection.selectAll('title').data(edge=>[edge]).join('title').text(edge=>`${text(edge.label)} · ${words(edge.status)}`);
    session.nodeSelection=svg.selectAll('.ipe-domain-nodes').data([null]).join('g').attr('class','ipe-domain-nodes').selectAll('g').data(nodes,node=>node.id).join(enter=>{
      const group=enter.append('g');group.append('rect').attr('width',nodeWidth).attr('height',nodeHeight).attr('rx',8);group.append('text').attr('class','ipe-domain-node-title').attr('x',12).attr('y',22);group.append('text').attr('class','ipe-domain-node-value').attr('x',12).attr('y',43);group.append('text').attr('class','ipe-domain-node-step').attr('x',12).attr('y',59);group.append('title');return group;
    }).attr('class',node=>`ipe-trace-node ipe-domain-node${node.focusLane==='fact'?' is-fact':''}${['discarded','failed','unavailable'].includes(node.status)?' has-failure':''}`)
      .attr('transform',node=>`translate(${positions.get(node.id).x},${positions.get(node.id).y})`).attr('tabindex',0).attr('role','button')
      .attr('aria-label',node=>`${text(node.label)}. ${words(node.status)}. ${Number.isInteger(node.step)?`Step ${node.step}`:'Exact step not recorded'}. Select for details.`)
      .on('click',(_,node)=>selectNode(session,node)).on('keydown',(event,node)=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();selectNode(session,node);}});
    session.nodeSelection.select('.ipe-domain-node-title').text(node=>short(node.focusLane==='fact'?(entity(node)?.predicate || node.label):node.label));
    session.nodeSelection.select('.ipe-domain-node-value').text(node=>short(node.focusLane==='fact'?JSON.stringify(entity(node)?.value ?? 'Value not retained'):node.focusLane==='execution'?(entity(node)?.conclusion || entity(node)?.status || node.status):'Declared operation',39));
    session.nodeSelection.select('.ipe-domain-node-step').text(node=>`${words(node.status)} · ${Number.isInteger(node.step)?`step ${node.step}`:'step not recorded'}`);
    session.nodeSelection.select('title').text(node=>text(node.label));
    session.previous.disabled=focus.page<=0;
    session.next.disabled=focus.page+1>=focus.pageCount;
    session.previous.hidden=session.next.hidden=focus.pageCount<=1;
    session.pageLabel.textContent=focus.allFactCount?(focus.pageCount<=1?`${focus.allFactCount} fact${focus.allFactCount===1?'':'s'} shown`:`Facts ${focus.page*6+1}–${focus.page*6+focus.facts.length} of ${focus.allFactCount}`):'No facts linked to this selection';
    session.pagination.hidden=!focus.allFactCount;
    session.empty.hidden=focus.allFactCount>0;
    session.empty.textContent=execution?'No facts are linked to this execution in the retained run. Its reported result does not establish requirement fulfillment.':'No facts are linked to this selection. Missing evidence is not proof that an operation succeeded.';
    const support=focus.support || {nodes:[],edges:[]};
    const reads=support.nodes.filter(node=>node.lane==='read');
    session.collectionDescription.textContent=`${reads.length} supporting native reads. Source records and collection gaps appear here, outside the operation diagram. Exact read links do not prove external execution or enforcement.`;
    session.issueList.replaceChildren(...focus.issues.map(issue=>el(doc,'li','',`${words(issue.code)}: ${text(issue.message)}`)));
    session.collectionBody.textContent=json({supporting_records:support.nodes.map(node=>({kind:node.kind,status:node.status,step:node.step,detail:node.detail})),links:support.edges});
    selectNode(session,nodes.find(node=>node.id===session.selected));
  }
  function render(container,run) {
    if(!container)return;
    let session=sessions.get(container);
    if(!session||session.root.parentElement!==container)session=create(container);
    const fingerprint=JSON.stringify(run ?? null);
    if(session.fingerprint===fingerprint&&session.available)return;
    session.fingerprint=fingerprint;
    session.heading.textContent=`Run ${run?.id ?? 'unknown'} · Operation evidence`;
    if(!globalThis.d3?.select||!globalThis.IpeTraceModel?.buildDomainFocus){
      session.available=false;session.state.textContent='The domain view did not load. Inspect the full run below.';session.viewport.replaceChildren();session.operationSelect.disabled=session.executionSelect.disabled=session.previous.disabled=session.next.disabled=true;return;
    }
    if(session.run?.id!==run?.id){session.selection={};session.selected=null;session.viewport.scrollTop=0;session.viewport.scrollLeft=0;}
    session.run=run;
    try{draw(session);session.available=true;}
    catch{session.available=false;session.state.textContent='The retained domain records could not be displayed. Inspect the full run below.';session.viewport.replaceChildren();}
  }
  globalThis.IpeTraceView=Object.freeze({render});
})();
