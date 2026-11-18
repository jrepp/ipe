(function(root,factory){const api=factory();if(typeof module==='object'&&module.exports)module.exports=api;else root.IpeFleet=api;})(typeof globalThis==='object'?globalThis:this,function(){
  'use strict';
  const ENTITY_COUNT=3500,POLICY_COUNT=500,PER_ENTITY=100,TOTAL=ENTITY_COUNT*PER_ENTITY,PAGE_SIZE=25;
  const teams=['Commerce','Platform','Identity','Data','Workplace','Finance','Customer','Infrastructure'];
  const families=['Accessibility','Security','Open-source legal','Build provenance','Test quality','Privacy','Change control','Reliability','Operations','Data retention'];
  const changes=['Source state','Policy rule','Artifact evidence','Exception revoked','Multiple changes'];
  const statusNames={met:'Passing',not_met:'Denied',unknown:'Unknown',error:'Error'};
  const pad=(n,size)=>String(n).padStart(size,'0');
  const entityId=e=>'ENT-'+pad(e+1,4),policyId=i=>'APP-'+pad(i+1,6);
  function hash(n){n=Math.imul(n^(n>>>16),0x45d9f3b);n=Math.imul(n^(n>>>16),0x45d9f3b);return (n^(n>>>16))>>>0;}
  const bucket=i=>i<1000?i%1000:hash(i+20261103)%1000;
  function status(i){const n=bucket(i);return n<940?'met':n<970?'not_met':n<990?'unknown':'error';}
  function transition(i){const n=bucket(i);return n>=940&&n<960?'new':n>=930&&n<940?'recovered':'stable';}
  function previous(i){return transition(i)==='new'?'met':transition(i)==='recovered'?'not_met':status(i);}
  function change(i){return transition(i)==='new'?i%5:-1;}
  function entity(e){return {id:entityId(e),index:e,name:teams[e%8]+' service '+pad(e+1,4),team:teams[e%8]};}
  const evaluationCount=i=>(1+hash(i+171)%12)*(1+Math.floor(i/PER_ENTITY)%4);
  function record(i){const e=Math.floor(i/PER_ENTITY),slot=i%100,definition=(slot+e*37)%POLICY_COUNT;return {id:policyId(i),policy_id:'POL-'+pad(definition+1,3),index:i,entity:entity(e),name:families[Math.floor(definition%100/10)]+' · rule '+(definition%10+1),status:status(i),previous:previous(i),transition:transition(i),change:change(i),evaluations:evaluationCount(i),owner:teams[e%8]+' controls',source:['ServiceNow','Policy registry','Build pipeline','Exception register','Multiple sources'][Math.max(0,change(i))]};}
  function filters(raw={}){
    return {q:String(raw.q||'').trim().toLowerCase().slice(0,120),team:teams.includes(raw.team)?raw.team:'',status:Object.hasOwn(statusNames,raw.status)?raw.status:'',change:changes.includes(raw.change)?raw.change:'',transition:['new','recovered','stable'].includes(raw.transition)?raw.transition:'',entity:/^ENT-\d{4}$/.test(raw.entity||'')?raw.entity:'',view:raw.view==='policies'?'policies':'entities',page:Math.max(1,Math.floor(Number(raw.page)||1))};
  }
  function query(raw={}){
    const f=filters(raw),counts={total:0,met:0,not_met:0,unknown:0,error:0,new:0,recovered:0,evaluations:0},edges={},groups=changes.map(()=>0),entityRows=new Map(),matches=[];
    let entityStart=0,entityEnd=ENTITY_COUNT;
    if(f.entity){entityStart=Number(f.entity.slice(4))-1;entityEnd=entityStart+1;if(entityStart<0||entityEnd>ENTITY_COUNT)return {filters:f,counts,edges,groups,rows:[],page:1,pages:1,rowCount:0,entityCount:0};}
    // Queries scan compact integer identities. Only the visible policy page becomes records.
    for(let e=entityStart;e<entityEnd;e++){
      if(f.team&&teams[e%8]!==f.team)continue;
      const ent=entity(e),entitySearch=(ent.id+' '+ent.name+' '+ent.team).toLowerCase();
      for(let slot=0;slot<PER_ENTITY;slot++){
        const i=e*PER_ENTITY+slot,st=status(i),tr=transition(i),ch=change(i);
        if(f.status&&f.status!==st||f.transition&&f.transition!==tr||f.change&&changes[ch]!==f.change)continue;
        if(f.q){const searchable=entitySearch+' '+policyId(i).toLowerCase()+' '+('POL-'+pad((slot+e*37)%POLICY_COUNT+1,3)).toLowerCase()+' '+families[Math.floor(((slot+e*37)%POLICY_COUNT)%100/10)].toLowerCase()+' rule '+(((slot+e*37)%POLICY_COUNT)%10+1);if(!f.q.split(/\s+/).every(term=>searchable.includes(term)))continue;}
        counts.total++;counts[st]++;counts.evaluations+=evaluationCount(i);if(tr!=='stable')counts[tr]++;
        const edge=previous(i)+'|'+st;edges[edge]=(edges[edge]||0)+1;if(ch>=0)groups[ch]++;
        if(!entityRows.has(e))entityRows.set(e,{...ent,total:0,met:0,not_met:0,unknown:0,error:0,new:0,evaluations:0});
        const row=entityRows.get(e);row.total++;row[st]++;row.evaluations+=evaluationCount(i);if(tr==='new')row.new++;
        if(f.view==='policies')matches.push(i);
      }
    }
    const rows=f.view==='entities'?[...entityRows.values()].sort((a,b)=>b.new-a.new||b.not_met-a.not_met||a.index-b.index):matches;
    const pages=Math.max(1,Math.ceil(rows.length/PAGE_SIZE)),page=Math.min(f.page,pages),visible=rows.slice((page-1)*PAGE_SIZE,page*PAGE_SIZE);
    return {filters:f,counts,edges,groups,rows:f.view==='policies'?visible.map(record):visible,page,pages,rowCount:rows.length,entityCount:entityRows.size};
  }
  function evaluate(input){if(!input||input.approval==null||input.findings==null||input.threshold==null||input.exception==null)return 'unknown';return input.approval==='approved'&&(input.exception||input.findings<=input.threshold)?'met':'not_met';}
  function comparison(index){
    if(!Number.isInteger(index)||index<0||index>=TOTAL)return null;
    const rec=record(index);
    if(rec.transition!=='new')return {record:rec,available:false,reason:rec.status==='met'?'This policy has no new failure in the sample window.':'No matching last-pass and first-failure pair is retained for this sample.'};
    const before={approval:'approved',findings:0,threshold:0,exception:false,artifact:'build-481',revision:'policy-r17',source:'record-r102'};
    const after={...before};let fields;
    switch(rec.change){
      case 0:after.approval='rejected';after.source='record-r103';fields=[['Source state',['approval','source']]];break;
      case 1:before.findings=2;before.threshold=5;Object.assign(after,{findings:2,threshold:0,revision:'policy-r18'});fields=[['Policy rule',['threshold','revision']]];break;
      case 2:after.findings=3;after.artifact='build-482';fields=[['Artifact evidence',['artifact','findings']]];break;
      case 3:before.findings=2;before.exception=true;Object.assign(after,{findings:2,exception:false,source:'record-r103'});fields=[['Exception state',['exception','source']]];break;
      default:Object.assign(before,{findings:2,threshold:5});Object.assign(after,{findings:8,threshold:1,revision:'policy-r18',artifact:'build-482'});fields=[['Policy rule',['threshold','revision']],['Artifact evidence',['artifact','findings']]];
    }
    return {record:rec,available:true,before,after,fields,passedAt:'2026-11-03T10:00:00Z',failedAt:'2026-11-03T10:05:00Z',observedAt:'2026-11-03T10:03:00Z',beforeResult:evaluate(before),afterResult:evaluate(after),scope:{entity:rec.entity.id,policy:rec.id,environment:'production'},rule:'Approval must be approved. Findings must not exceed the threshold unless an exception is active.'};
  }
  function replay(index,choice){const pair=comparison(index);if(!pair?.available)return null;const fields=choice==='all'?pair.fields.flatMap(f=>f[1]):pair.fields[Number(choice)]?.[1];if(!fields)return null;const input={...pair.after};for(const key of fields)input[key]=pair.before[key];return {input,result:evaluate(input),restored:fields};}
  return {ENTITY_COUNT,POLICY_COUNT,PER_ENTITY,TOTAL,PAGE_SIZE,teams,families,changes,statusNames,record,query,comparison,replay,evaluate};
});
