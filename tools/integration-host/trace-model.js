(function(root,factory){
  'use strict';
  const api=factory();
  if(typeof module==='object'&&module.exports)module.exports=api;
  else root.IpeTraceModel=api;
})(typeof globalThis!=='undefined'?globalThis:this,function(){
  'use strict';
  const MAX_NODES=180;
  const array=value=>Array.isArray(value)?value:[];
  const text=value=>typeof value==='string'?value:'';
  const step=value=>Number.isSafeInteger(value)&&value>0?value:undefined;
  const identity=value=>JSON.stringify([value?.kind,value?.key,value?.revision_id]);
  const control=value=>value&&[value.namespace,value.resource,value.operation,value.local_id].every(v=>typeof v==='string')?JSON.stringify([value.namespace,value.resource,value.operation,value.local_id]):null;
  const definition=source=>source?.definition&&[source.definition.namespace,source.definition.resource,source.definition.path,source.revision].every(v=>typeof v==='string')?JSON.stringify([source.definition.namespace,source.definition.resource,source.definition.path,source.revision]):null;
  const execution=value=>value&&[value.namespace,value.resource,value.run].every(v=>typeof v==='string')&&Number.isSafeInteger(value.attempt)&&value.attempt>0&&(value.job==null||typeof value.job==='string')&&(value.step==null||Number.isSafeInteger(value.step))?JSON.stringify([value.namespace,value.resource,value.run,value.attempt,value.job??null,value.step??null]):null;
  const append=(map,key,value)=>{if(key!==null){if(!map.has(key))map.set(key,[]);map.get(key).push(value);}};

  function traceModel(input,displayLimit){
    const run=input&&typeof input==='object'?input:{};
    const records=array(run.records),reads=array(run.evidence);
    const legacy=run.trace?.schema_version!==1;
    const emissions=legacy?[]:array(run.trace.emissions);
    const omittedEmissions=!legacy&&Number.isSafeInteger(run.trace.omitted)&&run.trace.omitted>0?run.trace.omitted:0;
    const nodes=[],edges=[],issues=[],receiptReads=new Map(),recordNodes=new Map(),emissionRows=new Map(),usedReceipts=new Set();
    const definitionNodes=new Map(),controlNodes=new Map(),executionNodes=new Map(),nodeRecords=new Map();
    const issue=(code,nodeId,message)=>issues.push({code,...(nodeId?{nodeId}:{}),message});
    const edge=(source,target,kind,label,status)=>{if(source!==target)edges.push({id:'edge:'+edges.length,source,target,kind,label,status});};
    for(const [index,read] of reads.entries()){
      if(!read||typeof read!=='object')continue;
      const id='read:'+index,status=text(read.status)||'unknown';
      nodes.push({id,lane:'read',label:text(read.operation)||'Unnamed read',kind:'read',status,detail:read,...(step(read.step)?{step:read.step}:{})});
      if(text(read.receipt))append(receiptReads,read.receipt,{id,read});
      if(status==='unavailable'||status==='failed')issue('read_failed',id,'Read failed'+(read.code?': '+String(read.code):''));
      if(read.truncated===true)issue('read_truncated',id,'Read response is incomplete.');
    }
    for(const emission of emissions){
      if(emission&&typeof emission==='object')append(emissionRows,identity(emission),emission);
    }
    for(const [index,record] of records.entries()){
      if(!record||typeof record!=='object')continue;
      const entity=record.model?.entity,id='record:'+index,kind=text(record.kind)||text(entity?.kind)||'legacy';
      const rows=emissionRows.get(identity(record))||[],emission=rows.length===1?rows[0]:null;
      const node={id,lane:kind==='fact'?'fact':'model',label:text(entity?.title)||text(entity?.requested)||text(entity?.predicate)||text(record.key)||'Untyped record',kind,status:'committed',detail:{record,...(emission?{emission}:{})},...(step(emission?.step)?{step:emission.step}:{})};
      nodes.push(node);nodeRecords.set(id,record);append(recordNodes,identity(record),node);
      if(rows.length>1)issue('ambiguous_emission',id,'More than one trace emission matches this record. Its emission step is unknown.');
      if(!legacy&&!rows.length&&!omittedEmissions)issue('missing_emission_trace',id,'No retained emission matches this record revision. Its emission step is unknown.');
      if(entity?.kind==='definition')append(definitionNodes,definition(entity.source),node);
      if(entity?.kind==='constraint')append(controlNodes,control(entity.control),node);
      if(entity?.kind==='outcome')append(executionNodes,execution(entity.execution),node);
    }
    const stagedSeen=new Set();
    for(const [index,emission] of emissions.entries()){
      if(!emission||typeof emission!=='object')continue;
      const key=identity(emission);
      if(recordNodes.has(key)||stagedSeen.has(key))continue;
      stagedSeen.add(key);
      const id='emission:'+index,kind=text(emission.kind)||'unknown';
      const status=run.status==='running'?'staged':['failed','interrupted'].includes(run.status)?'discarded':'unavailable';
      nodes.push({id,lane:kind==='fact'?'fact':'model',label:text(emission.key)||'Emitted record',kind,status,detail:{emission},...(step(emission.step)?{step:emission.step}:{})});
      if(status==='unavailable')issue('emitted_record_unavailable',id,'The emission has no matching record in this run.');
    }

    for(const node of nodes){
      if(node.lane==='read')continue;
      const owner=node.detail.record||node.detail.emission,receipts=array(owner.receipts);
      if(!receipts.length)issue('missing_receipt',node.id,'No supporting read receipt was retained for this record.');
      for(const receipt of new Set(receipts)){
        if(typeof receipt!=='string'||!receipt){issue('missing_receipt',node.id,'A supporting read receipt is missing or invalid.');continue;}
        const matches=receiptReads.get(receipt)||[];
        usedReceipts.add(receipt);
        if(matches.length!==1){issue(matches.length?'ambiguous_receipt':'missing_receipt',node.id,matches.length?'The receipt matches more than one read in this run.':'The supporting read is not retained in this run.');continue;}
        if(matches[0].read.status!=='observed'){
          issue('receipt_read_not_observed',node.id,'The referenced read has no successful observation.');continue;
        }
        edge(matches[0].id,node.id,'receipt','Supports record','resolved');
      }
    }
    // Check the complete retained input before applying display limits. Hidden nodes
    // must not turn exact references into apparent provenance gaps.
    for(const [index,read] of reads.entries()){
      if(read?.status==='observed'&&!read.truncated&&run.status==='succeeded'&&!omittedEmissions){
        if(!text(read.receipt))issue('read_receipt_missing','read:'+index,'The successful read has no retained receipt. Output linkage is unknown.');
        else if(!usedReceipts.has(read.receipt))issue('read_without_output','read:'+index,'No emitted record (may be a traversal read)');
      }
    }
    const sourceLink=(node,source,label,status='resolved',required=true)=>{
      const key=definition(source),matches=key===null?[]:definitionNodes.get(key)||[];
      if(matches.length===1)edge(matches[0].id,node.id,'model',label,status);
      else if(required)issue('unresolved_model_reference',node.id,matches.length?'The source definition is ambiguous in this run.':'The exact source definition and revision are not retained in this run.');
    };
    const nodeById=new Map(nodes.map(node=>[node.id,node]));
    for(const [id,record] of nodeRecords){
      const entity=record.model?.entity,node=nodeById.get(id);
      if(!entity)continue;
      if(entity.kind==='invocation'){
        sourceLink(node,entity.parent,'Parent definition');
        if(entity.callee)sourceLink(node,entity.callee,'Called definition',entity.resolution==='resolved'?'resolved':'candidate');
        else issue('unresolved_model_reference',id,'The called definition is unresolved.');
      }else if(entity.kind!=='definition'&&entity.source){
        // Source excerpts can name files with no interface Definition record.
        // Their receipt still provides exact provenance; absence is not a gap.
        sourceLink(node,entity.source,'Source definition','resolved',entity.kind!=='evidence');
      }
      if(entity.kind==='outcome'){
        sourceLink(node,entity.definition,'Execution definition');
        const candidates=array(entity.declaration_candidates),sourceKey=definition(entity.definition);
        if(!candidates.length)issue('uncertain_model_reference',id,'Execution-to-declaration correlation is '+(text(entity.correlation)||'unknown')+'; no declaration candidate was retained.');
        for(const candidate of candidates){
          const key=control(candidate),matches=(key===null?[]:controlNodes.get(key)||[]).filter(target=>sourceKey!==null&&definition(nodeRecords.get(target.id)?.model?.entity?.source)===sourceKey);
          if(matches.length===1)edge(matches[0].id,id,'model','Candidate declaration','candidate');
          else issue('unresolved_model_reference',id,matches.length?'The declaration candidate has multiple captured records.':'The declaration candidate and its source revision are not retained in this run.');
        }
      }
      if(entity.kind==='fact'&&entity.execution){
        const key=execution(entity.execution),matches=key===null?[]:executionNodes.get(key)||[];
        if(matches.length===1){
          edge(matches[0].id,id,'model','Same execution','resolved');
          const title=text(nodeRecords.get(matches[0].id)?.model?.entity?.title);
          if(title)node.label=title+' · '+(text(entity.predicate)||'Fact');
        }
        else issue('unresolved_model_reference',id,matches.length?'More than one retained outcome has this exact execution identity.':'The outcome for this exact execution identity is not retained in this run.');
      }
      if(entity.kind==='constraint')for(const dependency of array(entity.dependencies)){
        const key=control(dependency),matches=key===null?[]:controlNodes.get(key)||[];
        // A v1 dependency names a logical operation without a target revision.
        // Even a unique local match is a candidate, not proof of applicability.
        if(matches.length===1)edge(matches[0].id,id,'model','Declared dependency','candidate');
        else issue('unresolved_model_reference',id,matches.length?'The dependency has multiple captured revisions.':'The declared dependency is not retained in this run.');
      }
    }
    const counts={reads:0,models:0,facts:0,nodes:nodes.length,edges:edges.length,issues:issues.length};
    for(const node of nodes)counts[node.lane==='read'?'reads':node.lane==='fact'?'facts':'models']++;
    // Round-robin lanes preserve read, model, and fact visibility under the cap.
    const lanes=['read','model','fact'].map(lane=>nodes.filter(node=>node.lane===lane));
    const displayed=[],positions=[0,0,0];
    while(displayed.length<Math.min(displayLimit,nodes.length))for(let i=0;i<lanes.length&&displayed.length<displayLimit;i++)if(positions[i]<lanes[i].length)displayed.push(lanes[i][positions[i]++]);
    const visible=new Set(displayed.map(node=>node.id));
    const displayedEdges=edges.filter(link=>visible.has(link.source)&&visible.has(link.target));
    return {nodes:displayed,edges:displayedEdges,issues,counts,legacy,omitted:{nodes:nodes.length-displayed.length,edges:edges.length-displayedEdges.length,emissions:omittedEmissions}};
  }
  function buildTraceModel(run){return traceModel(run,MAX_NODES);}

  // A domain view starts with one declared operation and one observed execution.
  // Reads and definitions stay in support; they never become operating controls.
  function buildDomainFocus(run,selection={}){
    const trace=traceModel(run,Infinity),nodes=new Map(trace.nodes.map(node=>[node.id,node]));
    const operations=trace.nodes.filter(node=>node.detail.record?.model?.entity?.kind==='constraint').map(node=>{
      const entity=node.detail.record.model.entity;
      return {...node,operation:entity.control,actor:entity.actor_context,applicability:entity.applicability,declaredEnforcer:entity.enforcer,
        requirements:array(entity.obligations).map(item=>({id:item.id,description:item.description,specification:item.requirement,fulfillment:'unknown'}))};
    });
    const outcomes=trace.nodes.filter(node=>node.kind==='outcome');
    const allFacts=trace.nodes.filter(node=>node.lane==='fact');
    const candidateEdges=trace.edges.filter(edge=>edge.kind==='model'&&edge.label==='Candidate declaration');
    const executionEdges=trace.edges.filter(edge=>edge.kind==='model'&&edge.label==='Same execution'&&edge.status==='resolved');
    const mappedOutcomes=new Set(candidateEdges.map(edge=>edge.target)),mappedFacts=new Set(executionEdges.map(edge=>edge.target));
    const unlinkedOutcomes=outcomes.filter(node=>!mappedOutcomes.has(node.id)),orphanFacts=allFacts.filter(node=>!mappedFacts.has(node.id));
    const operation=selection.operationId==='__unmapped__'?null:operations.find(node=>node.id===selection.operationId)||operations[0]||null;
    const mode=operation?'operation':'unmapped';
    const permittedOutcomes=operation?new Set(candidateEdges.filter(edge=>edge.source===operation.id).map(edge=>edge.target)):null;
    const executions=operation?outcomes.filter(node=>permittedOutcomes.has(node.id)):unlinkedOutcomes;
    const onlyUnlinkedFacts=mode==='unmapped'&&selection.executionId==='__unlinked_facts__';
    const selectedExecution=onlyUnlinkedFacts?null:executions.find(node=>node.id===selection.executionId)||executions[0]||null;
    const relatedFacts=selectedExecution?new Set(executionEdges.filter(edge=>edge.source===selectedExecution.id).map(edge=>edge.target)):null;
    const availableFacts=selectedExecution?allFacts.filter(node=>relatedFacts.has(node.id)):mode==='unmapped'?orphanFacts:[];
    const pageCount=Math.max(1,Math.ceil(availableFacts.length/6));
    const page=Number.isSafeInteger(selection.factPage)?Math.min(pageCount-1,Math.max(0,selection.factPage)):0;
    const facts=availableFacts.slice(page*6,page*6+6);
    const focused=[operation,selectedExecution,...facts].filter(Boolean),focusedIds=new Set(focused.map(node=>node.id));
    const edges=trace.edges.filter(edge=>focusedIds.has(edge.source)&&focusedIds.has(edge.target)&&
      (edge.label==='Candidate declaration'||edge.label==='Same execution'));
    const supportIds=new Set(),queue=[...focusedIds];
    // Retain incoming provenance and declaration dependencies as detail only.
    const incoming=new Map();
    for(const edge of trace.edges)append(incoming,edge.target,edge.source);
    while(queue.length){
      const id=queue.pop();
      for(const source of incoming.get(id)||[])if(!focusedIds.has(source)&&!supportIds.has(source)){supportIds.add(source);queue.push(source);}
    }
    if(!focused.length)for(const node of trace.nodes)if(node.lane==='read'||node.kind==='definition'||node.kind==='evidence')supportIds.add(node.id);
    const contextIds=new Set([...focusedIds,...supportIds]);
    return {operations,executions,operation,execution:selectedExecution,facts,allFactCount:availableFacts.length,page,pageCount,edges,
      issues:trace.issues.filter(issue=>!issue.nodeId||contextIds.has(issue.nodeId)),legacy:trace.legacy,omitted:trace.omitted,mode,
      support:{nodes:[...supportIds].map(id=>nodes.get(id)),edges:trace.edges.filter(edge=>contextIds.has(edge.source)&&contextIds.has(edge.target)&&!edges.includes(edge))},
      unmapped:{executions:unlinkedOutcomes.length,facts:orphanFacts.length},
      selection:{operationId:operation?.id||'__unmapped__',executionId:selectedExecution?.id||(mode==='unmapped'?'__unlinked_facts__':null),factPage:page}};
  }
  return {buildTraceModel,buildDomainFocus};
});
