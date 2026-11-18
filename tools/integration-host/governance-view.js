/* Same-capture governance descriptions. Source claims never grant authority. */
(() => {
  'use strict';
  const sessions = new WeakMap();
  const array = value => Array.isArray(value) ? value : [];
  const words = value => String(value ?? 'Unknown').replaceAll('_', ' ');
  const stable = value => JSON.stringify(value && typeof value === 'object' ? Array.isArray(value) ? value.map(normalize) : normalize(value) : value);
  function normalize(value) { return value && typeof value === 'object' ? Array.isArray(value) ? value.map(normalize) : Object.fromEntries(Object.keys(value).sort().map(key=>[key,normalize(value[key])])) : value; }
  const same = (a,b) => a != null && b != null && stable(a) === stable(b);
  const entity = record => record?.model?.entity;
  const governance = record => entity(record)?.kind === 'governance' ? entity(record).record : null;
  const key = item => 'governance:'+JSON.stringify([item.type,item.id.namespace,item.id.authority,item.id.local_id]);
  const operationKey = id => 'control:'+JSON.stringify([id.namespace,id.resource,id.operation,id.local_id]);
  const refs = item => item.type==='policy'?[item.authority]:item.type==='contract'?array(item.parties):item.type==='control'?array(item.governing_rules):item.type==='handoff'?[item.policy,item.from,item.to]:item.type==='role_assignment'?[item.principal,item.role]:item.type==='contact_method'?[item.owner]:[];
  function el(doc,tag,text,className) { const node=doc.createElement(tag);if(text!=null)node.textContent=String(text);if(className)node.className=className;return node; }
  function specification(doc, requirement) {
    const node=el(doc,'div',null,'ipe-governance-specification');
    node.append(el(doc,'strong',requirement.description),el(doc,'p',`Requirement ID: ${requirement.id}`));
    const text=typeof requirement.specification==='string'?requirement.specification:JSON.stringify(requirement.specification??'');
    let parsed;
    try { parsed=JSON.parse(text); } catch { /* Opaque and truncated specifications remain source text. */ }
    if(parsed && typeof parsed==='object' && !Array.isArray(parsed)){
      const fields=Object.entries(parsed),dl=el(doc,'dl');
      for(const [name,value] of fields.slice(0,16)){
        dl.append(el(doc,'dt',name),el(doc,'dd',value===null?'null (source value)':typeof value==='string'?value:JSON.stringify(value)));
      }
      node.append(dl,el(doc,'p','Field names and values come from the retained specification. Their presence does not prove enforcement.'));
      if(fields.length>16)node.append(el(doc,'p',`Showing 16 of ${fields.length} fields. Open the retained specification for the rest.`));
      const raw=el(doc,'details'),limited=text.length>16384?text.slice(0,16384)+'\nDisplay limited to 16 KiB.':text;raw.append(el(doc,'summary','Retained specification'),el(doc,'pre',limited));node.append(raw);
    }else node.append(el(doc,'p',text),el(doc,'small','Source text. No structured interpretation is available.'));
    return node;
  }
  function identity(record) { return JSON.stringify([record?.key,record?.revision_id]); }
  function drawNativeStory(doc, story, data, run, session, redraw) {
    const nodes=array(data.nodes), links=array(data.links), rules=array(data.rules);
    const byId=new Map(nodes.map(node=>[identity(node.record),node]));
    const label=node=>String(node?.title||node?.record?.key||'Unknown record').slice(0,200);
    const displayValue=value=>{const text=JSON.stringify(value)??'Unknown';return text.length>512?text.slice(0,512)+'… (value shortened)':text;};
    const appendReason=(parent,link)=>{if(!link?.reason)return;const detail=el(doc,'details');detail.dataset.key=`coverage-link:${session.nativeRule}:${link.kind}:${identity(link.from)}:${identity(link.to)}`;detail.open=session.openDetails.includes(detail.dataset.key);detail.append(el(doc,'summary','Why this link?'),el(doc,'p',String(link.reason).slice(0,1024)));parent.append(detail);};
    const maxOperations=6,maxExecutions=6,maxFacts=6;
    const header=el(doc,'div',null,'ipe-coverage-header');
    header.append(el(doc,'h3','Rule and evidence'),el(doc,'p',`Run ${data.run_id} · ${words(data.run_status)}.`));
    header.append(el(doc,'p','A link describes retained records. It does not prove that a rule applied or that a requirement was met.'));
    const ruleSelect=el(doc,'select',null,'ipe-native-rule-select');ruleSelect.setAttribute('aria-label','Rule to explain');
    for(const rule of rules){const node=byId.get(identity(rule.record)),option=el(doc,'option',`${words(rule.kind||'rule')} · ${label(node)}`);option.value=identity(rule.record);ruleSelect.append(option);}
    const selectedRule=rules.find(rule=>identity(rule.record)===session.nativeRule)||rules[0];
    if(selectedRule){session.nativeRule=identity(selectedRule.record);ruleSelect.value=session.nativeRule;}
    const ruleLabel=el(doc,'label','Rule to explain');ruleLabel.append(ruleSelect);header.append(ruleLabel);
    const allRequirements=array(selectedRule?.requirements),requirements=allRequirements.slice(0,32),reqSelect=el(doc,'select',null,'ipe-native-requirement-select');reqSelect.setAttribute('aria-label','Requirement to explain');
    for(const requirement of requirements){const option=el(doc,'option',requirement.description||requirement.id||'Requirement');option.value=String(requirement.id);reqSelect.append(option);}
    const selectedRequirement=requirements.find(requirement=>String(requirement.id)===session.nativeRequirement)||requirements[0];
    if(selectedRequirement){session.nativeRequirement=String(selectedRequirement.id);reqSelect.value=session.nativeRequirement;}
    if(requirements.length){const requirementLabel=el(doc,'label','Requirement to explain');requirementLabel.append(reqSelect);header.append(requirementLabel);}
    story.append(header);
    const ruleNode=byId.get(identity(selectedRule?.record));
    const requirementSection=el(doc,'section',null,'ipe-coverage-step');requirementSection.append(el(doc,'h3','1. What is required?'));
    if(selectedRequirement){
      requirementSection.append(el(doc,'strong',String(selectedRequirement.description||selectedRequirement.id||'Requirement').slice(0,300)),el(doc,'p',`Fulfillment: ${words(selectedRequirement.fulfillment)}. ${String(selectedRequirement.reason||'').slice(0,240)}`));
      const text=String(selectedRequirement.specification??'');
      let fields;try{fields=JSON.parse(text);}catch{/* Opaque source remains literal. */}
      if(fields&&typeof fields==='object'&&!Array.isArray(fields)){
        const values=Object.entries(fields).filter(([name,value])=>!['id','node_id'].includes(name)&&typeof value!=='object');
        const summary=el(doc,'dl');for(const [name,value] of values.slice(0,4))summary.append(el(doc,'dt',words(name)),el(doc,'dd',typeof value==='boolean'?(value?'Yes':'No'):String(value).slice(0,240)));
        requirementSection.append(summary);
      }else if(text)requirementSection.append(el(doc,'p',text.length>512?text.slice(0,512)+'… (open source requirement for the rest)':text));
      if(selectedRequirement.specification!=null){const detail=el(doc,'details');detail.dataset.key=`coverage-spec:${identity(selectedRule.record)}:${selectedRequirement.id}`;detail.open=session.openDetails.includes(detail.dataset.key);detail.append(el(doc,'summary','Source requirement'),specification(doc,{id:selectedRequirement.id,description:selectedRequirement.description||'Source requirement',specification:selectedRequirement.specification}));requirementSection.append(detail);}
      if(ruleNode)appendNodeFreshness(doc,requirementSection,ruleNode,`coverage-node:${identity(selectedRule.record)}:rule`,session.openDetails);
    }else requirementSection.append(el(doc,'p','No requirement was retained for this rule.'));
    const operationSection=el(doc,'section',null,'ipe-coverage-step');operationSection.append(el(doc,'h3','2. Which operation is linked?'));
    const ruleLinks=links.filter(link=>link.kind==='rule_subject'&&identity(link.from)===identity(selectedRule?.record));
    if(!ruleLinks.length)operationSection.append(el(doc,'p','No operation link was retained for this rule.'));
    for(const link of ruleLinks.slice(0,maxOperations)){
      const operation=byId.get(identity(link.to));
      const item=el(doc,'p',`${link.resolution==='exact'?'Exact match':'Possible match'} · ${label(operation)}`);operationSection.append(item);appendReason(operationSection,link);
      if(operation)appendNodeFreshness(doc,operationSection,operation,`coverage-node:${identity(selectedRule.record)}:operation:${identity(link.to)}`,session.openDetails);
    }
    if(ruleLinks.length>maxOperations)operationSection.append(el(doc,'p',`Showing ${maxOperations} of ${ruleLinks.length} operation links.`));
    const executionSection=el(doc,'section',null,'ipe-coverage-step');executionSection.append(el(doc,'h3','3. What happened?'));
    let executionCount=0,factCount=0;
    for(const ruleLink of ruleLinks.slice(0,maxOperations)){
      for(const executionLink of links.filter(link=>link.kind==='operation_execution'&&identity(link.from)===identity(ruleLink.to)).slice(0,maxExecutions)){
        if(executionCount++>=maxExecutions)break;
        const execution=byId.get(identity(executionLink.to));
        const outcome=el(doc,'article',null,'ipe-coverage-outcome');
        outcome.append(el(doc,'strong',label(execution)),el(doc,'p',executionLink.resolution==='exact'?'Exact execution link':'Possible execution link'));appendReason(outcome,executionLink);
        if(execution?.status||execution?.conclusion)outcome.append(el(doc,'p',`Provider result: ${words(execution.conclusion||execution.status)}.`));
        if(execution)appendNodeFreshness(doc,outcome,execution,`coverage-node:${identity(selectedRule.record)}:execution:${identity(executionLink.to)}`,session.openDetails);
        const facts=links.filter(link=>link.kind==='execution_fact'&&identity(link.from)===identity(executionLink.to));
        if(!facts.length)outcome.append(el(doc,'p','No facts were linked to this exact execution.'));
        for(const factLink of facts.slice(0,maxFacts)){
          if(factCount++>=maxFacts)break;
          const fact=byId.get(identity(factLink.to));
          outcome.append(el(doc,'p',`${factLink.resolution==='exact'?'Same execution':'Possible observation'} · ${label(fact)}`));appendReason(outcome,factLink);
          if(fact?.predicate)outcome.append(el(doc,'p',`${words(fact.predicate)}: ${displayValue(fact.value)}`));
          if(fact?.expired===true)outcome.append(el(doc,'p','Expired or unusable evidence.'));
          if(fact)appendNodeFreshness(doc,outcome,fact,`coverage-node:${identity(selectedRule.record)}:fact:${identity(factLink.to)}`,session.openDetails);
        }
        if(facts.length>maxFacts)outcome.append(el(doc,'p',`Showing ${maxFacts} of ${facts.length} fact links.`));
        executionSection.append(outcome);
      }
    }
    if(!executionCount)executionSection.append(el(doc,'p','No execution candidate was linked to these operations.'));
    if(executionCount>=maxExecutions)executionSection.append(el(doc,'p',`Showing at most ${maxExecutions} execution links.`));
    if(factCount>maxFacts)executionSection.append(el(doc,'p',`Showing at most ${maxFacts} observations. Inspect retained records for the rest.`));
    if(allRequirements.length>requirements.length)requirementSection.append(el(doc,'p',`Showing ${requirements.length} of ${allRequirements.length} requirements.`));
    const conclusion=el(doc,'section',null,'ipe-coverage-conclusion');
    conclusion.append(el(doc,'h3','Was the requirement met?'),el(doc,'strong','We cannot tell from this capture.'),el(doc,'p','No recorded check connects this requirement to the evidence. A successful run alone does not prove the requirement was met.'));
    story.append(requirementSection,operationSection,executionSection,conclusion);
    const gaps=array(data.gaps),omissions=data.omissions||{};
    if(gaps.length){const details=el(doc,'details',null,'ipe-coverage-gaps');details.append(el(doc,'summary',`Missing evidence and limits · ${gaps.length}`));for(const gap of gaps.slice(0,32))details.append(el(doc,'p',`${words(gap.kind)}${gap.record?.key?` · ${gap.record.key}`:''}: ${gap.reason||'Reason not provided'}`));if(gaps.length>32)details.append(el(doc,'p',`Showing 32 of ${gaps.length} items.`));story.append(details);}
    const omitted=['nodes','links','rules','gaps'].filter(name=>Number.isFinite(omissions[name])&&omissions[name]>0).map(name=>`${omissions[name]} ${name}`);
    if(omitted.length)story.append(el(doc,'p',`The host omitted ${omitted.join(', ')} from this bounded explanation.`,'ipe-coverage-omissions'));
    ruleSelect.onchange=()=>{session.nativeRule=ruleSelect.value;session.nativeRequirement=null;session.openDetails=[];redraw();story.ownerDocument.querySelector('.ipe-native-rule-select')?.focus({preventScroll:true});};
    reqSelect.onchange=()=>{session.nativeRequirement=reqSelect.value;session.openDetails=[];redraw();story.ownerDocument.querySelector('.ipe-native-requirement-select')?.focus({preventScroll:true});};
  }
  function appendNodeFreshness(doc, parent, node, key, openDetails=[]) {
    const detail=el(doc,'details');detail.dataset.key=key;detail.open=openDetails.includes(key);detail.append(el(doc,'summary','Source, freshness, and collection details'));
    const times=[];
    if(Number.isFinite(node.observed_at))times.push(`Observed ${new Date(node.observed_at*1000).toISOString()}`);
    if(Number.isFinite(node.expires_at))times.push(`Expires ${new Date(node.expires_at*1000).toISOString()}`);
    if(node.expired===true)times.push('Expired or unusable evidence');
    if(node.temporal_state)times.push(`Temporal state: ${words(node.temporal_state)}`);
    if(node.valid_from!=null)times.push(`Valid from ${node.valid_from}`);
    if(node.valid_until!=null)times.push(`Valid until ${node.valid_until}`);
    if(node.source)times.push(`Source ${typeof node.source==='string'?node.source:JSON.stringify(node.source)}`);
    if(node.claim)times.push(`Source claim: ${words(node.claim)}`);
    for(const value of times)detail.append(el(doc,'p',value));
    if(array(node.receipts).length){detail.append(el(doc,'p',`Collection receipts: ${node.receipts.slice(0,8).join(', ')}`));if(node.receipts.length>8)detail.append(el(doc,'p',`Showing 8 of ${node.receipts.length} receipts.`));}
    parent.append(detail);
  }
  function render(container, run, installation = null, explanation = {status:'missing'}) {
    if (!container) return;
    const now=Math.floor(Date.now()/1000);
    const temporal=array(run?.records).map(record=>{const item=governance(record);return [record.expires_at,item?.valid_until].some(value=>Number.isFinite(value)&&value<=now)||Number.isFinite(item?.valid_from)&&item.valid_from>now;});
    const fingerprint=JSON.stringify([run ?? null,temporal,installation,explanation]), previous=sessions.get(container);
    if(previous?.fingerprint===fingerprint)return;
    const openDetails=previous?.runId===run?.id?[...container.querySelectorAll('.ipe-coverage-story details[open][data-key]')].map(detail=>detail.dataset.key):[];
    const focused=container.contains(container.ownerDocument.activeElement)?container.ownerDocument.activeElement:null;
    const focusConcept=focused?.matches('button[data-concept]')?focused.dataset.concept:null;
    const focusClass=['ipe-governance-select','ipe-governance-requirement-select','ipe-ontology-filter','ipe-native-rule-select','ipe-native-requirement-select'].find(name=>focused?.classList.contains(name));
    const doc=container.ownerDocument, root=el(doc,'section',null,'ipe-governance');
    const session={fingerprint,runId:run?.id,selected:previous?.runId===run?.id?previous.selected:null,requirement:previous?.runId===run?.id?previous.requirement:null,nativeRule:previous?.runId===run?.id?previous.nativeRule:null,nativeRequirement:previous?.runId===run?.id?previous.nativeRequirement:null,openDetails,concept:previous?.runId===run?.id?previous.concept:null,gapsOnly:previous?.runId===run?.id?previous.gapsOnly:false,technicalOpen:previous?.runId===run?.id?previous.technicalOpen:false};sessions.set(container,session);
    const all=array(run?.records), records=all.slice(0,512), entries=records.filter(governance).map(record=>({record,item:governance(record)}));
    const guide=el(doc,'details',null,'ipe-governance-guide');guide.append(el(doc,'summary','Capture scope and mapping states'));
    guide.append(el(doc,'p','Select a concept to inspect its source and next check. The map groups concepts; it does not imply links between adjacent cards.'));
    const help=el(doc,'details');help.append(el(doc,'summary','What the mapping states mean'),el(doc,'p','Source claims describe configured rules. Exact references connect retained records. Candidate links need more evidence. A missing mapping is not a violation.'),el(doc,'p','These source descriptions do not verify authority, requirement fulfillment, or active enforcement. Historical records do not establish current authority.'));guide.append(help);
    const inventory=el(doc,'p');
    inventory.textContent=['policy','contract','control','handoff','principal','contact_method'].map(type=>`${words(type)}: ${entries.filter(entry=>entry.item.type===type).length}`).join(' · ');
    guide.append(inventory,el(doc,'p','Counts describe this capture only. Zero means no description was retained.'));
    const settings=el(doc,'details',null,'ipe-governance-settings');settings.append(el(doc,'summary',installation?.snapshot?'Collector runtime configuration · retained snapshot':'Collector runtime configuration · installed now'));
    settings.append(el(doc,'p','These settings control collection scope and evidence lifetime. They do not define external policy or activate IPE enforcement.'));
    if(installation?.package){
      const dl=el(doc,'dl'),pkg=installation.package||{};
      for(const [name,value] of [['Collector version',installation.adapter_digest||'Not retained'],['Read grants',array(pkg.operations).join(', ')||'None listed'],['Output grants',array(pkg.output_kinds).join(', ')||'None listed'],['Schedule',`${pkg.enabled?'Scheduled':'Manual only'} · interval ${pkg.interval_seconds??'Unknown'} seconds`],['Fact lifetime',`${pkg.fact_ttl_seconds??'Unknown'} seconds`]])dl.append(el(doc,'dt',name),el(doc,'dd',value));
      settings.append(dl,el(doc,'p',installation.snapshot?'These settings were saved with this snapshot. They do not establish the current installation.':'These are current installation settings. Use Changes to compare them with a retained capture.'));
    }else settings.append(el(doc,'p','Installation settings are not included. Inspect Program for current input, read grants, schedule, and fact lifetime.'));
    guide.append(settings);
    const collectionGaps=array(run?.summary?.gaps);
    if(collectionGaps.length){
      const disclosure=el(doc,'details',null,'ipe-governance-collection-gaps');
      disclosure.open=!entries.length;
      disclosure.append(el(doc,'summary',`Collection gaps · ${collectionGaps.length}`));
      for(const gap of collectionGaps.slice(0,40))disclosure.append(el(doc,'p',`${words(gap.code)}${gap.path?' · '+gap.path:''}`));
      root.append(disclosure);
    }
    if(all.length>records.length)root.append(el(doc,'p',`Inspection limited to ${records.length} of ${all.length} records. Missing links can result from this limit.`));
    if(run?.status!=='succeeded')root.append(el(doc,'p','This capture did not commit records. Retained emissions do not establish captured governance.','ipe-governance-warning'));
    if(!entries.length){
      root.append(el(doc,'p','No governance descriptions were retained in this capture.'));
      const story=el(doc,'section',null,'ipe-coverage-story');story.setAttribute('aria-label','Rule and evidence explanation');
      const statusHeader=el(doc,'div',null,'ipe-coverage-header');statusHeader.append(el(doc,'h3','Rule and evidence'));
      if(explanation?.status==='loading'||(explanation?.status==='refreshing'&&!explanation?.data))statusHeader.append(el(doc,'p','Loading explanation…'));
      else if(explanation?.status==='error')statusHeader.append(el(doc,'p',`The shared explanation could not be loaded. ${explanation.message||'Refresh to try again.'} Retained records remain available below.`,'ipe-governance-warning'));
      else if(explanation?.status==='run-unavailable')statusHeader.append(el(doc,'p','This run is no longer available for explanation. Select another retained run. Retained records remain available below.','ipe-governance-warning'));
      else if(explanation?.status==='missing')statusHeader.append(el(doc,'p','This saved snapshot does not include this explanation. The browser will not infer these links. Inspect retained records below.','ipe-governance-warning'));
      else if(explanation?.status==='mismatch'||(explanation?.data&&explanation.data.run_id!==run?.id))statusHeader.append(el(doc,'p','This explanation belongs to another run. The browser will not use its links. Inspect retained records below.','ipe-governance-warning'));
      else if(explanation?.status==='unsupported'||explanation?.data?.schema!=='ipe.integration.coverage-explanation.v1')statusHeader.append(el(doc,'p','This host returned an unsupported explanation version. The browser will not infer these links. Inspect retained records below.','ipe-governance-warning'));
      else drawNativeStory(doc,story,explanation.data,run,session,()=>{});
      if(!story.querySelector('.ipe-coverage-header'))story.prepend(statusHeader);
      root.append(story);
      const rawRecords=el(doc,'details');rawRecords.append(el(doc,'summary',`Raw supporting records · ${all.length}`),el(doc,'pre',JSON.stringify(all.slice(0,32),null,2)));
      if(all.length>32)rawRecords.append(el(doc,'p',`Showing 32 of ${all.length} records.`));root.append(rawRecords,guide);container.replaceChildren(root);return;
    }
    const rules=entries.filter(({item})=>['policy','contract'].includes(item.type));
    const label=el(doc,'label','Rule to inspect'), select=el(doc,'select',null,'ipe-governance-select');select.setAttribute('aria-label','Rule to inspect');
    for(const entry of rules){const option=el(doc,'option',`${words(entry.item.type)} · ${entry.item.title}`);option.value=key(entry.item)+'@'+entry.item.source.revision;select.append(option);}
    if(!rules.length){const option=el(doc,'option','No policy or contract captured');option.value='';select.append(option);select.disabled=true;}
    if([...select.options].some(option=>option.value===session.selected))select.value=session.selected;
    label.append(select);const body=el(doc,'div',null,'ipe-governance-body');root.append(body,guide);container.replaceChildren(root);
    function resolve(ref){const matches=entries.filter(({item})=>ref && item.type===ref.kind && same(item.id,ref.id) && item.source.revision===ref.source_revision);return {matches,status:matches.length>1?'Ambiguous reference':matches.length===0?'Target not captured at this source revision':ref.resolution==='resolved'?'Exact retained reference':'Candidate reference'};}
    function reference(ref){if(!ref)return 'Reference unavailable';const result=resolve(ref),name=result.matches.length===1?(result.matches[0].item.title||result.matches[0].item.id.local_id):ref.id?.local_id;return `${name ?? 'Unknown'} · ${result.status}`;}
    function field(dl,name,value){dl.append(el(doc,'dt',name),el(doc,'dd',value));}
    function card(entry){const {record,item}=entry,article=el(doc,'article',null,'ipe-governance-record');article.append(el(doc,'h4',item.title||words(item.type)+' · '+item.id.local_id));const dl=el(doc,'dl');field(dl,'Record type',words(item.type));field(dl,'Claim',words(record.model?.claim));
      if(item.type==='principal') {field(dl,'Principal kind',words(item.principal_kind));field(dl,'Source identity',`${item.id.namespace} / ${item.id.authority} / ${item.id.local_id}`);}
      if(item.authority)field(dl,'Declared authority',reference(item.authority));
      if(item.parties)for(const party of item.parties)field(dl,'Declared party',reference(party));
      if(item.type==='control'){field(dl,'Mechanism description',item.mechanism);for(const ref of item.governing_rules)field(dl,'Governing rule',reference(ref));field(dl,'Verification','Operating enforcement has not been verified.');}
      if(item.type==='handoff'){field(dl,'Governing policy',reference(item.policy));field(dl,'From',reference(item.from));field(dl,'To',reference(item.to));for(const name of ['trigger','required_action','completion','continuation'])field(dl,words(name),item[name]);}
      if(item.type==='role_assignment'){field(dl,'Principal',reference(item.principal));field(dl,'Role',reference(item.role));field(dl,'Declared relationship',words(item.relationship));}
      if(item.type==='contact_method'){field(dl,'Owner',reference(item.owner));field(dl,'Channel',words(item.channel));field(dl,'Source-provided contact',item.endpoint||'Not captured');field(dl,'Availability',words(item.availability));field(dl,'Purpose',item.purpose);}
      const now=Math.floor(Date.now()/1000),end=[record.expires_at,item.valid_until].filter(value=>Number.isFinite(value));
      if(end.length)field(dl,'Temporal state',Math.min(...end)<=now?'Expired · historical description only':'Retained expiry has not passed · current authority is not verified');
      if(item.valid_from!=null && item.valid_from>now)field(dl,'Validity','Not yet valid');
      field(dl,'Source revision',item.source.revision);field(dl,'Collected at',Number.isFinite(record.observed_at)&&Number.isFinite(new Date(record.observed_at*1000).getTime())?new Date(record.observed_at*1000).toISOString():'Not retained');article.append(dl);
      for(const requirement of array(item.requirements)){const requirementNode=el(doc,'div',null,'ipe-governance-requirement');requirementNode.append(el(doc,'strong',requirement.description),el(doc,'p',requirement.specification),el(doc,'p','Requirement fulfillment: Unknown'));article.append(requirementNode);}
      const support=el(doc,'details',null,'ipe-governance-support'),receipts=array(record.receipts),reads=array(run?.evidence).filter(read=>read.status==='observed'&&receipts.includes(read.receipt));support.append(el(doc,'summary',`Extraction support · ${reads.length} successful native reads / ${receipts.length} receipt references`),el(doc,'p','Receipts support collection provenance. Shared receipts do not prove a semantic link or requirement fulfillment.'));
      const missing=receipts.filter(receipt=>!reads.some(read=>read.receipt===receipt));if(!receipts.length||missing.length)support.append(el(doc,'p',!receipts.length?'Extraction support gap: no receipt references retained.':`Extraction support gap: ${missing.length} receipt references have no successful native read in this capture.`,'ipe-governance-warning'));
      for(const read of reads.slice(0,32))support.append(el(doc,'p',`${read.operation} · ${read.step==null?'Step not retained':'Step '+read.step} · ${read.receipt}`));article.append(support);
      const raw=el(doc,'details',null,'ipe-governance-raw');raw.append(el(doc,'summary','Raw retained record'),el(doc,'pre',JSON.stringify(record,null,2).length>32768?JSON.stringify(record,null,2).slice(0,32768)+'\nDisplay limited to 32 KiB. Inspect the full retained run for this record.':JSON.stringify(record,null,2)));article.append(raw);return article;
    }
    function draw(){session.technicalOpen=body.querySelector('.ipe-coverage-technical')?.open??session.technicalOpen;const open=[...body.querySelectorAll('.ipe-coverage-story details[open][data-key]')].map(detail=>detail.dataset.key);if(open.length)session.openDetails=open;if(session.selected!==select.value){session.requirement=null;session.concept=null;session.gapsOnly=false;session.technicalOpen=false;}session.selected=select.value;body.replaceChildren();const selected=rules.find(({item})=>key(item)+'@'+item.source.revision===select.value);
      if(selected){const description=el(doc,'details',null,'ipe-governance-description');description.append(el(doc,'summary','Full policy or contract description'),card(selected));body.append(description);}
      const related=selected?entries.filter(({item})=>['control','handoff'].includes(item.type)&&refs(item).some(ref=>ref.kind===selected.item.type&&same(ref.id,selected.item.id)&&ref.source_revision===selected.item.source.revision)):[];
      const mechanisms=el(doc,'details',null,'ipe-governance-mechanisms');mechanisms.append(el(doc,'summary',`Mechanism and handoff details · ${related.length}`));if(!related.length)mechanisms.append(el(doc,'p','No mechanism or handoff reference to this rule was captured at this source revision.'));for(const entry of related.slice(0,32))mechanisms.append(card(entry));
      const principals=new Map();for(const entry of [selected,...related].filter(Boolean))for(const ref of refs(entry.item))if(ref.kind==='principal')for(const target of resolve(ref).matches)principals.set(key(target.item),target);
      const assignments=entries.filter(({item})=>item.type==='role_assignment'&&[item.principal,item.role].some(ref=>[...principals.values()].some(({item:target})=>ref.kind===target.type&&same(ref.id,target.id)&&ref.source_revision===target.source.revision)));
      for(const entry of assignments)for(const ref of refs(entry.item))for(const target of resolve(ref).matches)principals.set(key(target.item),target);
      const contacts=entries.filter(({item})=>item.type==='contact_method'&&[...principals.values()].some(({item:target})=>same(item.owner.id,target.id)&&item.owner.source_revision===target.source.revision));
      const people=el(doc,'details',null,'ipe-governance-people');people.append(el(doc,'summary',`Principal and contact details · ${principals.size} principals / ${contacts.length} contacts`));for(const entry of [...principals.values(),...assignments,...contacts].slice(0,32))people.append(card(entry));if(!principals.size)people.append(el(doc,'p','No referenced principal was retained at the required source revision.'));
      const overlay=el(doc,'details',null,'ipe-governance-overlay');overlay.append(el(doc,'summary','Observed external execution'),el(doc,'p','Policy applicability at execution time: Unknown. Collection time does not establish when this rule took effect.','ipe-governance-applicability'));
      const operations=records.filter(record=>entity(record)?.kind==='constraint'&&array(selected?.item.subjects).includes(operationKey(entity(record).control)));
      const outcomes=records.filter(record=>entity(record)?.kind==='outcome'&&operations.some(operation=>{const op=entity(operation),outcome=entity(record);return same(op.source.definition,outcome.definition?.definition)&&op.source.revision===outcome.definition?.revision&&array(outcome.declaration_candidates).some(id=>same(id,op.control));}));
      if(!outcomes.length)overlay.append(el(doc,'p','No captured governance-to-execution link. Matching names and shared read receipts do not establish this link.'));
      for(const record of outcomes.slice(0,32)){const outcome=entity(record),facts=records.filter(record=>entity(record)?.kind==='fact'&&same(entity(record).execution,outcome.execution));overlay.append(el(doc,'h5',outcome.title),el(doc,'p',`Candidate execution link · ${words(outcome.conclusion||outcome.status)}`),el(doc,'p',`${facts.length} facts have this exact execution identity. Execution success does not establish requirement fulfillment.`));for(const fact of facts.slice(0,6))overlay.append(el(doc,'p',`${entity(fact).predicate}: ${JSON.stringify(entity(fact).value)}${fact.expires_at!=null&&fact.expires_at<=Date.now()/1000?' · Expired':''}`));}
      let requirementChoice=null;
      const mapping=el(doc,'section',null,'ipe-governance-mapping');
      const heading=el(doc,'div',null,'ipe-ontology-heading');
      heading.append(el(doc,'h3','Policy ontology'));
      const filter=el(doc,'button','Show gaps and unknowns','ipe-ontology-filter');filter.type='button';filter.setAttribute('aria-pressed',String(session.gapsOnly));heading.append(filter);
      mapping.append(heading,el(doc,'p','Select a concept to inspect its mapping. Adjacent cards do not establish a relationship.','ipe-ontology-caption'));
      const wrap=el(doc,'div',null,'ipe-ontology-map');wrap.setAttribute('role','group');wrap.setAttribute('aria-label','Policy ontology concepts');
      const inspector=el(doc,'section',null,'ipe-ontology-inspector');inspector.setAttribute('aria-label','Selected concept details');
      const groups=new Map();mapping.append(wrap,inspector);
      function sourceDetail(record){
        const detail=el(doc,'details'),source=governance(record)?.source||entity(record)?.source||entity(record)?.definition;
        detail.append(el(doc,'summary','Inspect source and emission'));
        if(source?.definition)detail.append(el(doc,'p',`${source.definition.resource} / ${source.definition.path} · pointer ${source.pointer||'(root)'} · revision ${source.revision}`));
        const reads=array(run?.evidence).filter(read=>read.status==='observed'&&array(record.receipts).includes(read.receipt));
        for(const read of reads.slice(0,8))detail.append(el(doc,'p',`Native read: ${read.operation} · step ${read.step??'not retained'}${read.truncated?' · Partial response':''}`));
        if(!reads.length)detail.append(el(doc,'p','No successful native read is linked by receipt in this capture.'));
        const emissions=array(run?.trace?.emissions).filter(emission=>record.key!=null&&record.revision_id!=null&&emission.key===record.key&&emission.revision_id===record.revision_id&&emission.kind===record.kind);
        detail.append(el(doc,'p',emissions.length?`Record emitted at collector step ${emissions.map(e=>e.step).join(', ')}.`:'Emission step not retained for this record.'));
        if(record.model?.claim)detail.append(el(doc,'p',`Claim: ${words(record.model.claim)}`));
        return detail;
      }
      function row(concept,basis,status,next,record){
        if(!groups.has(concept))groups.set(concept,[]);
        const detail=el(doc,'article',null,'ipe-ontology-detail');detail.dataset.concept=concept;
        const item=governance(record),expired=[record?.expires_at,item?.valid_until].some(value=>Number.isFinite(value)&&value<=now);
        if(expired)status=`Expired evidence · ${status}`;
        else if(Number.isFinite(item?.valid_from)&&item.valid_from>now)status=`Not yet valid · ${status}`;
        detail.append(el(doc,'strong',status,'ipe-ontology-state'),typeof basis==='string'?el(doc,'p',basis):basis,el(doc,'p',`Next check: ${next}`));
        if(record)detail.append(sourceDetail(record));
        const uncertain=!['Source description','Exact retained reference','Subject identity matched','Execution identity matched'].includes(status);
        groups.get(concept).push({detail,status,uncertain});
      }
      if(selected){
        row(words(selected.item.type),selected.item.title+' · The collector maps this source to a governing rule.','Source description','Confirm scope and authority in the source before treating this rule as applicable.',selected.record);
        const requirements=array(selected.item.requirements);
        if(requirements.length){
          const focus=el(doc,'label','Requirement to inspect'),choice=el(doc,'select',null,'ipe-governance-requirement-select');choice.setAttribute('aria-label','Requirement to inspect');
          for(const [index,requirement] of requirements.entries()){const option=el(doc,'option',requirement.description);option.value=String(index);choice.append(option);}
          if(requirements.some(requirement=>requirement.id===session.requirement))choice.value=String(requirements.findIndex(requirement=>requirement.id===session.requirement));
          const requirement=requirements[Number(choice.value)];session.requirement=requirement.id;
          choice.onchange=()=>{session.concept='Requirement';session.requirement=requirements[Number(choice.value)].id;draw();body.querySelector('.ipe-governance-requirement-select')?.focus();};
          focus.append(choice);
          requirementChoice=focus;
          row('Requirement',specification(doc,requirement),'Fulfillment unknown','Confirm how this source condition maps to the selected requirement. The retained pointer identifies the source record only.',selected.record);
          groups.get('Requirement')[0].detail.prepend(focus);
          const binding=el(doc,'details',null,'ipe-governance-binding');binding.append(el(doc,'summary','Requirement-to-fact binding: Not retained'));
          binding.append(el(doc,'h4','Does execution evidence establish this requirement?'),el(doc,'p',`Selected requirement: ${requirement.description}`),el(doc,'strong','Requirement-to-fact binding: Not retained'),el(doc,'p','The governance capture does not bind this requirement to a fact predicate or an evidence check.'),el(doc,'p','Execution facts below share an execution identity. They do not establish that this requirement was checked or fulfilled.'),el(doc,'p','Next check: identify the evidence predicate, expected value, policy revision, and execution context needed to assess this condition.'));
          groups.get('Requirement')[0].detail.append(binding);
        }
        if(!requirements.length)row('Requirement','No named requirement retained for this rule.','Mapping gap','Inspect the source for conditions that the collector does not represent.');
      }
      const controls=related.filter(entry=>entry.item.type==='control');
      for(const entry of controls.slice(0,6)){const link=refs(entry.item).find(ref=>selected&&ref.kind===selected.item.type&&same(ref.id,selected.item.id)&&ref.source_revision===selected.item.source.revision);row('Control',entry.item.mechanism,resolve(link).status,'Obtain evidence that this mechanism evaluated the selected requirement during execution.',entry.record);}
      if(!controls.length)row('Control','No mechanism references the selected rule at this source revision.','Mapping gap','Inspect source enforcement settings and retain a mechanism-to-rule reference.');
      for(const operation of operations.slice(0,6))row('Operation',`${entity(operation).title} · Its operation identity occurs in the rule subjects.`,'Subject identity matched','Check provider scope conditions. Identity alone does not establish applicability.',operation);
      const unmatched=array(selected?.item.subjects).filter(subject=>!operations.some(record=>operationKey(entity(record).control)===subject));
      if(!operations.length||unmatched.length)row('Operation scope',unmatched.length?unmatched.slice(0,6).join(' · '):'No operation subject was mapped.','Mapping gap','Resolve provider scope to operation identities. Do not join records by display name.');
      for(const outcomeRecord of outcomes.slice(0,6)){
        row('External outcome',`${entity(outcomeRecord).title} · The definition revision matches; the captured model lists this operation as a candidate.`,'Candidate execution link','Bind the applicable policy revision and scope to this execution.',outcomeRecord);
        const facts=records.filter(record=>entity(record)?.kind==='fact'&&same(entity(record).execution,entity(outcomeRecord).execution));
        for(const fact of facts.slice(0,6))row('Integration fact',`${entity(fact).predicate}: ${JSON.stringify(entity(fact).value)} · Exact execution identity.`,fact.expires_at!=null&&fact.expires_at<=now?'Expired evidence':'Execution identity matched','Inspect the emission step and expiry. This fact is not automatically an IPE runtime fact.',fact);
        if(!facts.length)row('Integration fact','No fact has this exact execution identity.','Evidence gap','Collect execution-bound facts before checking the requirement.');
      }
      if(!outcomes.length)row('External outcome','No execution matches the captured operation and definition revision.','Evidence gap','Collect outcomes with definition revisions and operation candidates.');
      row('Policy applicability','Collection time does not establish when the rule took effect.','Unknown','Obtain the policy revision and validity period that governed this execution.');
      for(const entry of related.filter(({item})=>item.type==='handoff').slice(0,6)){
        row('Handoff',`${entry.item.trigger} → ${reference(entry.item.to)} → ${entry.item.continuation}`,
          refs(entry.item).map(ref=>resolve(ref).status).find(status=>status!=='Exact retained reference')||'Exact retained reference','Confirm the recipient, completion evidence, and continuation in the source.',entry.record);
      }
      const principalRefs=[...new Map([selected,...related].filter(Boolean).flatMap(entry=>refs(entry.item)).filter(ref=>ref?.kind==='principal').map(ref=>[stable(ref),ref])).values()];
      for(const ref of principalRefs.slice(0,6)){
        const result=resolve(ref);row('Principal',reference(ref),result.status,'Confirm authority and role validity in the source.',result.matches.length===1?result.matches[0].record:null);
      }
      if(!principalRefs.length)row('Principal','No authority or party reference was retained.','Mapping gap','Inspect the governing source for its authority or parties.');
      const lanes=[['Governing rule',['policy','contract','Requirement']],['Mechanisms and people',['Control','Handoff','Principal']],['Operation scope',['Operation','Operation scope']],['Execution evidence',['External outcome','Integration fact','Policy applicability']]];
      const nodes=new Map();
      for(const [title,concepts] of lanes){
        const lane=el(doc,'section',null,'ipe-ontology-lane');lane.append(el(doc,'h4',title));wrap.append(lane);
        for(const concept of concepts){
          const items=groups.get(concept);if(!items)continue;
          const button=el(doc,'button',null,'ipe-ontology-node');button.type='button';button.dataset.concept=concept;
          const uncertain=items.filter(item=>item.uncertain).length;
          button.dataset.state=uncertain?'uncertain':'retained';
          button.append(el(doc,'strong',words(concept)),el(doc,'span',items.length===1?items[0].status:`${items.length} mappings · ${uncertain} uncertain`));
          button.onclick=()=>{session.concept=concept;inspect();if(doc.defaultView.matchMedia?.('(max-width:1099px)').matches)inspector.scrollIntoView?.({block:'start'});};lane.append(button);nodes.set(concept,button);
        }
      }
      function inspect(){
        const available=[...groups.keys()].filter(concept=>!session.gapsOnly||groups.get(concept).some(item=>item.uncertain));
        if(!available.includes(session.concept))session.concept=available.includes('Requirement')?'Requirement':available[0];
        for(const [concept,button] of nodes){button.hidden=!available.includes(concept);button.setAttribute('aria-pressed',String(concept===session.concept));}
        for(const lane of wrap.children)lane.hidden=![...lane.querySelectorAll('button')].some(button=>!button.hidden);
        inspector.replaceChildren(el(doc,'h4',session.concept?words(session.concept):'No gaps in the displayed mappings'));
        for(const [concept,items] of groups)for(const {detail} of items){detail.hidden=concept!==session.concept;inspector.append(detail);}
        filter.setAttribute('aria-pressed',String(session.gapsOnly));
      }
      filter.onclick=()=>{session.gapsOnly=!session.gapsOnly;inspect();};inspect();
      mapping.append(el(doc,'p','Shows up to six mappings per concept and six facts per outcome, from the first 512 records. Inspect Records for the full capture.','ipe-ontology-limit'));
      body.prepend(mapping);
      body.append(overlay);
      const gaps=entries.flatMap(entry=>refs(entry.item).map(ref=>({entry,ref,result:resolve(ref)}))).filter(({result})=>result.status!=='Exact retained reference');
      const disclosure=el(doc,'details',null,'ipe-governance-gaps');disclosure.append(el(doc,'summary',`Capture-wide reference gaps and candidates · ${gaps.length}`));for(const {entry,ref} of gaps.slice(0,64))disclosure.append(el(doc,'p',`${entry.item.title||entry.item.id.local_id}: ${reference(ref)}`));body.append(disclosure);
      const summary=el(doc,'details',null,'ipe-governance-handoff-summary');summary.append(el(doc,'summary','Handoffs and source contacts'));
      for(const {item,record} of related.filter(({item})=>item.type==='handoff').slice(0,6))summary.append(el(doc,'p',`${item.trigger} → ${reference(item.to)} → ${item.continuation} · Claim: ${words(record.model?.claim)}`));
      summary.append(el(doc,'p',`${principals.size} referenced principals · ${contacts.length} source contact routes. These descriptions do not establish current authority.`));
      const routes=el(doc,'ul');for(const {item,record} of contacts.slice(0,6)){const expired=[record.expires_at,item.valid_until].some(value=>Number.isFinite(value)&&value<=Date.now()/1000);routes.append(el(doc,'li',`${reference(item.owner)} · ${words(item.channel)}: ${item.endpoint||'Contact not captured'} · ${words(item.availability)}${expired?' · Expired — historical route only':''} · Claim: ${words(record.model?.claim)}`));}summary.append(routes);body.append(summary,mechanisms,people);
      const inventory=el(doc,'details',null,'ipe-governance-inventory');inventory.append(el(doc,'summary',`All captured governance descriptions · ${entries.length}`));for(const entry of entries.slice(0,32))inventory.append(card(entry));if(entries.length>32)inventory.append(el(doc,'p','Showing the first 32 descriptions. Inspect the full retained run for additional records.'));body.append(inventory);

      // The native projection is the only source for the default relationship story.
      const technical=el(doc,'details',null,'ipe-coverage-technical');
      technical.open=session.technicalOpen===true;
      technical.append(el(doc,'summary','Supporting record map and source data'));
      technical.append(el(doc,'p','This supporting map is derived in the browser from retained records. Use the shared explanation above for run-specific relationship status.'));
      technical.append(label);
      for(const child of [...body.children])technical.append(child);
      technical.ontoggle=()=>{if(technical.isConnected)session.technicalOpen=technical.open;};
      const collectionDetails=root.querySelector('.ipe-governance-collection-gaps');
      if(collectionDetails)technical.append(collectionDetails);
      technical.append(guide);
      const nativeStory=el(doc,'section',null,'ipe-coverage-story');nativeStory.setAttribute('aria-label','Rule and evidence explanation');
      const statusHeader=el(doc,'div',null,'ipe-coverage-header');statusHeader.append(el(doc,'h3','Rule and evidence'));
      if(explanation?.status==='loading'||(explanation?.status==='refreshing'&&!explanation?.data))statusHeader.append(el(doc,'p','Loading explanation…'));
      else if(explanation?.status==='error')statusHeader.append(el(doc,'p',`The shared explanation could not be loaded. ${explanation.message||'Refresh to try again.'} Retained records remain available below.`,'ipe-governance-warning'));
      else if(explanation?.status==='run-unavailable')statusHeader.append(el(doc,'p','This run is no longer available for explanation. Select another retained run. Retained records remain available below.','ipe-governance-warning'));
      else if(explanation?.status==='missing')statusHeader.append(el(doc,'p','This saved snapshot does not include this explanation. The browser will not infer these links. Inspect retained records below.','ipe-governance-warning'));
      else if(explanation?.status==='mismatch'||(explanation?.data&&explanation.data.run_id!==run?.id))statusHeader.append(el(doc,'p','This explanation belongs to another run. The browser will not use its links. Inspect retained records below.','ipe-governance-warning'));
      else if(explanation?.status==='unsupported'||explanation?.data?.schema!=='ipe.integration.coverage-explanation.v1')statusHeader.append(el(doc,'p','This host returned an unsupported explanation version. The browser will not infer these links. Inspect retained records below.','ipe-governance-warning'));
      else drawNativeStory(doc,nativeStory,explanation.data,run,session,draw);
      if(!nativeStory.querySelector('.ipe-coverage-header'))nativeStory.prepend(statusHeader);
      body.replaceChildren(nativeStory,technical);
    }
    select.onchange=draw;draw();
    if(focusConcept)[...body.querySelectorAll('button[data-concept]')].find(button=>button.dataset.concept===focusConcept&&!button.hidden)?.focus({preventScroll:true});
    else if(focusClass)root.querySelector('.'+focusClass)?.focus({preventScroll:true});
  }
  globalThis.IpeGovernanceView={render};
})();
