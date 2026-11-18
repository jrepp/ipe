// Synthetic, receipt-bound product ship qualification collector. It performs no I/O.
const SOURCE = {namespace:'ipe-demo',resource:'product-ship',path:'release-qualification.json'};
const REVISION = 'synthetic-qualification-v1';
const NAMESPACE = 'ipe-demo';
const AUTHORITY = 'synthetic-qualification-broker';
function id(kind, local) { return {namespace:NAMESPACE,authority:AUTHORITY,local_id:kind==='root'?local:kind+':'+local}; }
function source(pointer) { return {definition:SOURCE,revision:REVISION,pointer}; }
function ref(kind, local) { return {kind,id:id(kind,local),resolution:'resolved',source_revision:REVISION}; }
function step(context,responses,state) {
  if (state && state.phase==='complete') return ipe.complete(state.summary);
  if (state === null) return ipe.exchange([ipe.call('qualification','read_release_qualification',{release_id:context.input.release.id})],{phase:'read'});
  let snapshot,receipt,offset=0;
  if (state.phase==='emit') {
    ({snapshot,receipt,offset}=state);
  } else {
    const response=ipe.response(responses,'qualification');
    if (response.code !== undefined) return ipe.complete({status:'unknown',reason:'provider_read_failed',gaps:[{code:'provider_read_failed'}],publication:false});
    const rows=ipe.records(responses,'qualification');
    if (rows.length!==1 || !rows[0].release || !rows[0].policy || !Array.isArray(rows[0].policies) || !Array.isArray(rows[0].evidence)) return ipe.complete({status:'unknown',reason:'provider_response_invalid',gaps:[{code:'provider_response_invalid'}],publication:false});
    snapshot=rows[0];
    receipt=response.receipt;
  }
  const release=snapshot.release,root=snapshot.policy;
  let policies=snapshot.policies,evidence=snapshot.evidence;
  if (typeof context.input.policy_id==='string') {
    policies=policies.filter(policy=>policy.id===context.input.policy_id);
    if (policies.length!==1) return ipe.complete({status:'unknown',reason:'child_policy_not_found',gaps:[{code:'child_policy_not_found'}],publication:false});
    const selected=new Set(policies[0].controls.map(control=>control.id));
    evidence=evidence.filter(item=>selected.has(item.control_id));
  }
  const observations=[];
  let index=0;
  const selected=()=>{const current=index++;return current>=offset&&current<offset+8;};
  if (selected()) {
    const owner={type:'principal',id:id('principal','release-governance'),title:'Release governance owners',principal_kind:'group',source:source('/owners')};
    observations.push(ipe.observation(ipe.governanceModel(owner,{scope:'release:'+release.id,claim:'provider_reported'}),{owner_count:policies.length},[receipt]));
  }
  const rootRecord={type:'policy',id:id('root',root.id),title:root.title,authority:ref('principal','release-governance'),subjects:['release:'+release.id],requirements:root.children.map(childId=>({id:childId,description:'Required child policy '+childId,specification:JSON.stringify({child_policy_id:childId})})),source:source('/policies/'+root.id)};
  for (const policy of policies) {
    if (selected()) {
      const governancePolicy={type:'policy',id:id('policy',policy.id),title:policy.title,authority:ref('principal','release-governance'),subjects:['release:'+release.id],requirements:policy.controls.map(control=>({id:control.id,description:control.title,specification:control.description})),source:source('/policies/'+policy.id)};
      observations.push(ipe.observation(ipe.governanceModel(governancePolicy,{scope:'release:'+release.id,claim:'declared'}),{parent_id:policy.parent_id,owner:policy.owner},[receipt]));
    }
    for (const control of policy.controls) {
      if (!selected()) continue;
      const governanceControl={type:'control',id:id('control',control.id),title:control.title,governing_rules:[ref('policy',policy.id)],mechanism:'Synthetic release qualification check',source:source('/controls/'+control.id)};
      observations.push(ipe.observation(ipe.governanceModel(governanceControl,{scope:'release:'+release.id,claim:'declared'}),{parent_id:policy.id,source:control.source},[receipt]));
    }
  }
  if (selected()) observations.push(ipe.observation(ipe.governanceModel(rootRecord,{scope:'release:'+release.id,claim:'declared'}),{children:root.children},[receipt]));
  for (const row of evidence) {
    if (!selected()) continue;
    const value={control_id:row.control_id,release_id:row.release_id,artifact:row.artifact,environment:row.environment,status:row.status,reason:row.reason,observed_at:row.observed_at,expires_at:row.expires_at,source:row.source};
    observations.push(ipe.observation({schema_version:1,scope:'release:'+release.id,claim:'provider_reported',entity:{kind:'fact',subject:row.release_id+'#'+row.control_id,predicate:'product_ship.control_status',value:row.status,authority:AUTHORITY,execution:null}},value,[receipt]));
  }
  const summary={status:'succeeded',release_id:release.id,controls_observed:evidence.length,publication:false,gaps:[]};
  const next=offset+observations.length;
  // Keep compact source data between steps; construct only the current batch.
  return ipe.exchange([],next<index?{phase:'emit',snapshot,receipt,offset:next}:{phase:'complete',summary},observations);
}
