// Synthetic ServiceNow responsibility and contact extraction. It performs no I/O.
const NAMESPACE='ipe-demo';
const AUTHORITY='synthetic-servicenow-provenance';
const SOURCE={namespace:'ipe-demo',resource:'product-ship',path:'provenance.json'};
function id(kind,local){return {namespace:NAMESPACE,authority:AUTHORITY,local_id:kind+':'+local};}
function source(pointer,revision){return {definition:SOURCE,revision,pointer};}
function ref(kind,local,revision,resolution='resolved'){
  return {kind,id:id(kind,local),resolution,source_revision:resolution==='resolved'?revision:null};
}
function seconds(value){const parsed=ipe.timestamp(value);return parsed===null?null:Math.floor(parsed/1000);}
function step(context,responses,state){
  if(state&&state.phase==='complete')return ipe.complete(state.summary);
  if(state===null)return ipe.exchange([ipe.call('context','read_control_context',{control_id:context.input.control_id})],{phase:'read'});
  let snapshot,snapshotJson,receipt,offset=0;
  if(state.phase==='emit'){
    ({snapshotJson,receipt,offset}=state);
    snapshot=JSON.parse(snapshotJson);
  }else{
    const result=ipe.response(responses,'context');
    if(result.code!==undefined)return ipe.complete({status:'unavailable',control_id:context.input.control_id,observed_at:context.input.observed_at,receipt:null,publication:false,gaps:[{code:'provider_read_failed'}]});
    const rows=ipe.records(responses,'context');
    if(rows.length!==1||!rows[0].context||rows[0].context.control_id!==context.input.control_id)
      return ipe.complete({status:'unavailable',control_id:context.input.control_id,observed_at:context.input.observed_at,receipt:result.receipt||null,publication:false,gaps:[{code:'control_context_missing'}]});
    snapshot=rows[0].context;
    receipt=result.receipt;
    snapshot.collection.source_receipt=snapshot.collection.receipt;
    snapshot.collection.receipt=result.receipt;
    for(const record of snapshot.records){
      record.source_receipt=record.receipt||null;
      record.receipt=result.receipt;
    }
    if(result.truncated===true){
      snapshot.collection.status='partial';
      if(!snapshot.collection.gaps.some(gap=>gap.code==='record_limit'))snapshot.collection.gaps.push({code:'record_limit'});
      for(const record of snapshot.records)record.status='unavailable';
    }
    snapshotJson=JSON.stringify(snapshot);
    return ipe.exchange([],{phase:'emit',snapshotJson,receipt,offset:0});
  }
  const observations=[],revision=snapshot.records[0]?.source_revision||'synthetic-servicenow-provenance-v1';
  if(offset===0){
    const sourceModel={schema_version:1,scope:'control:'+snapshot.control_id,claim:'provider_reported',entity:{kind:'evidence',source:source('/controls/'+snapshot.control_id,revision),method:'provider_document',description:'Synthetic ServiceNow record context and sourced responsibility/profile data.'}};
    const evidence=ipe.observation(sourceModel,{control_id:snapshot.control_id,context_snapshot:snapshot},[receipt]);
    return ipe.exchange([],{phase:'emit',snapshotJson,receipt,offset:1},[evidence]);
  }
  let index=1;
  const selected=()=>{const current=index++;return current>=offset&&current<offset+4;};
  const add=(record,pointer,data)=>{if(selected())observations.push(ipe.observation(ipe.governanceModel({...record,source:source(pointer,revision)},{scope:'control:'+snapshot.control_id,claim:'provider_reported'}),data,[receipt]));};
  const peopleById=new Map();
  for(const person of snapshot.people){
    const matches=peopleById.get(person.id)||[];
    matches.push(person);
    peopleById.set(person.id,matches);
  }
  const people=new Map([...peopleById].filter(([,matches])=>matches.length===1).map(([personId,matches])=>[personId,matches[0]]));
  const roleIds=new Map();
  for(const assignment of snapshot.assignments){
    const roleId='role:'+assignment.role.toLowerCase().replace(/[^a-z0-9]+/g,'-');
    roleIds.set(assignment.role,roleId);
  }

  for(const person of people.values()){
    const personRecord={type:'principal',id:id('principal',person.id),title:person.name,principal_kind:'human'};
    add(personRecord,'/people/'+person.id,{control_id:snapshot.control_id,kind:'person',person});
  }
  for(const [role,roleId] of roleIds){
    add({type:'principal',id:id('principal',roleId),title:role,principal_kind:'role'},'/roles/'+roleId,{control_id:snapshot.control_id,kind:'role',role});
  }
  for(const assignment of snapshot.assignments){
    const person=people.get(assignment.person_id);
    const resolved=assignment.resolution==='resolved'&&!!person;
    const principalRef=ref('principal',assignment.person_id,revision,resolved?'resolved':'unknown');
    const roleRef=ref('principal',roleIds.get(assignment.role),revision);
    add({type:'role_assignment',id:id('role_assignment',assignment.id),principal:principalRef,role:roleRef,relationship:'responsibility',valid_from:seconds(assignment.observed_at),valid_until:seconds(assignment.expires_at)},'/assignments/'+assignment.id,{control_id:snapshot.control_id,kind:'assignment',assignment});
    if(!person)continue;
    for(const [channel,contact] of [['email',person.email],['provider_profile',person.slack]]){
      const available=contact.status==='available';
      const endpoint=channel==='email'?contact.value:contact.url;
      add({type:'contact_method',id:id('contact_method',assignment.id+':'+channel),owner:ref('principal',person.id,revision),channel,endpoint:available?endpoint:null,availability:available?'available':contact.status==='unavailable'?'unavailable':'unknown',purpose:channel==='email'?'Source-provided email contact route; delivery is not verified.':'Source-provided Slack profile route; delivery is not verified.',valid_until:seconds(contact.expires_at)},'/people/'+person.id+'/'+channel,{control_id:snapshot.control_id,kind:'contact',person_id:person.id,contact});
    }
  }
  const summary={status:snapshot.collection.status,control_id:snapshot.control_id,observed_at:context.input.observed_at,receipt,record_count:snapshot.records.length,assignment_count:snapshot.assignments.length,publication:false,gaps:snapshot.collection.gaps};
  const next=offset+observations.length;
  return ipe.exchange([],next<index?{phase:'emit',snapshotJson,receipt,offset:next}:{phase:'complete',summary},observations);
}
