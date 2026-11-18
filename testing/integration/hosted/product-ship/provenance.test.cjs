const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const dir=__dirname;
const fixture=JSON.parse(fs.readFileSync(path.join(dir,'provenance.json'),'utf8'));
const {replayContext,collectProvenance}=require('./provenance-runner.cjs');

test('synthetic ServiceNow contexts cover all 18 checks with canonical record identity and explicit state labels',()=>{
  const result=collectProvenance();
  assert.equal(result.schema,'ipe.product-ship-provenance.v1');
  assert.equal(result.simulated,true);
  assert.equal(result.contexts.length,18);
  assert.equal(new Set(result.contexts.map(item=>item.control_id)).size,18);
  for(const context of result.contexts){
    assert.equal(context.records.length,1);
    const record=context.records[0];
    assert.equal(record.system,'ServiceNow');
    assert.match(record.sys_id,/^[0-9a-f]{32}$/);
    assert.equal(new URL(record.url).searchParams.get('sys_id'),record.sys_id);
    assert.equal(record.required_state,'closed_complete');
    assert.equal(record.observed_state,'closed_complete');
    assert.equal(record.status,'available');
    assert.ok(context.assignments.length>=2);
    assert.ok(context.assignments.every(item=>item.source_record_id===record.id));
    assert.ok(context.people.every(person=>/^[0-9a-f]{32}$/.test(person.system_id)));
    for(const person of context.people) {
      if(person.slack.url) assert.equal(new URL(person.slack.url).pathname.split('/').pop(),person.slack.user_id);
    }
  }
  assert.ok(fixture.state_labels.closed_complete.includes('not a universal ServiceNow state code'));
  assert.equal(result.contexts[0].people[0].email.status,'unavailable');
  assert.equal(result.contexts[0].people[0].email.value,null);
  assert.equal(result.contexts[1].assignments.find(item=>item.role==='Reviewer').resolution,'unresolved');
  assert.ok(!result.contexts[1].people.some(item=>item.id==='person-control-reviewer-unresolved'));
});

test('SDK collector emits bounded receipt-bound governance descriptions without implying approval occurred',()=>{
  const capture=replayContext(fixture.contexts[0].control_id);
  assert.equal(capture.summary.status,'available');
  assert.equal(capture.requests.length,1);
  assert.deepEqual(capture.requests[0].params,{control_id:fixture.contexts[0].control_id});
  assert.ok(capture.observations.length<=16);
  assert.ok(capture.observations.length>0);
  assert.equal(capture.observations.filter(item=>item.model.schema_version===1&&item.model.entity.kind==='evidence').length,1);
  const rawEvidence=capture.observations.find(item=>item.model.entity.kind==='evidence');
  assert.equal(rawEvidence.data.context_snapshot.records[0].sys_id,fixture.contexts[0].records[0].sys_id);
  for(const observation of capture.observations.filter(item=>item.model.schema_version===2)){
    assert.equal(observation.model.entity.kind,'governance');
    assert.equal(observation.receipts.length,1);
    if(observation.model.entity.record.type==='role_assignment')
      assert.equal(observation.model.entity.record.relationship,'responsibility');
  }
  const assignment=capture.observations.find(item=>item.model.schema_version===2&&item.model.entity.record.type==='role_assignment');
  assert.equal(assignment.model.entity.record.valid_from,Math.floor(Date.parse(fixture.as_of)/1000));
  const unknownEmail=capture.observations.find(item=>item.model.schema_version===2&&item.model.entity.record.type==='contact_method'&&item.model.entity.record.channel==='email'&&item.model.entity.record.availability==='unknown');
  assert.equal(unknownEmail,undefined);
  const unavailableEmail=capture.observations.find(item=>item.model.schema_version===2&&item.model.entity.record.type==='contact_method'&&item.model.entity.record.channel==='email'&&item.model.entity.record.availability==='unavailable');
  assert.ok(unavailableEmail);
  assert.equal(unavailableEmail.model.entity.record.endpoint,null);
  const receipt=capture.observations[0].receipts[0];
  assert.ok(receipt.startsWith('fixture:broker:context:'));
  assert.ok(capture.observations.every(item=>item.receipts[0]===receipt));
  assert.equal(rawEvidence.data.context_snapshot.records[0].receipt,receipt);
  assert.equal(rawEvidence.data.context_snapshot.collection.receipt,receipt);
  assert.equal(rawEvidence.data.context_snapshot.records[0].source_receipt,fixture.contexts[0].records[0].receipt);
});

test('denied, truncated, missing, and dangling-person broker cases keep gaps explicit',()=>{
  const control=fixture.contexts[0].control_id;
  const denied=replayContext(control,{denied:true});
  assert.equal(denied.summary.status,'unavailable');
  assert.equal(denied.observations.length,0);
  assert.ok(denied.summary.gaps.some(gap=>gap.code==='provider_read_failed'));

  const truncated=replayContext(control,{truncated:true});
  assert.equal(truncated.summary.status,'partial');
  assert.ok(truncated.summary.gaps.some(gap=>gap.code==='record_limit'));
  assert.equal(truncated.observations.find(item=>item.data.context_snapshot).data.context_snapshot.records[0].status,'unavailable');

  const missing=replayContext(control,{missing:true});
  assert.equal(missing.summary.status,'unavailable');
  assert.equal(missing.observations.length,0);
  assert.ok(missing.summary.gaps.some(gap=>gap.code==='control_context_missing'));

  const dangling=replayContext(control,{danglingPerson:true});
  const assignment=dangling.observations.find(item=>item.model.schema_version===2&&item.model.entity.record.type==='role_assignment'&&item.model.entity.record.principal.resolution==='unknown');
  assert.ok(assignment);
  assert.equal(assignment.model.entity.record.relationship,'responsibility');
  assert.ok(!dangling.observations.some(item=>item.model.schema_version===2&&item.model.entity.record.type==='principal'&&item.model.entity.record.id.local_id==='principal:person-control-reviewer'));

  const duplicated=replayContext(control,{duplicatePerson:true});
  const ambiguousAssignment=duplicated.observations.find(item=>item.model.schema_version===2&&item.model.entity.record.type==='role_assignment'&&item.model.entity.record.principal.id.local_id==='principal:person-release-owner');
  assert.equal(ambiguousAssignment.model.entity.record.principal.resolution,'unknown');
  assert.ok(!duplicated.observations.some(item=>item.model.schema_version===2&&item.model.entity.record.type==='contact_method'&&item.model.entity.record.owner.id.local_id==='principal:person-release-owner'));
});

test('provider source receipts cannot replace the native receipt attached to an emission',()=>{
  const capture=replayContext(fixture.contexts[0].control_id,{sourceReceipt:'provider:receipt-forgery'});
  const evidence=capture.observations.find(item=>item.model.schema_version===1&&item.model.entity.kind==='evidence');
  const nativeReceipt=evidence.receipts[0];
  const record=evidence.data.context_snapshot.records[0];
  assert.equal(record.source_receipt,'provider:receipt-forgery');
  assert.equal(record.receipt,nativeReceipt);
  assert.equal(evidence.data.context_snapshot.collection.receipt,nativeReceipt);
});

test('combined product ship runner adds captured provenance contexts for the single-pane view',()=>{
  const {run}=require('./runner.cjs');
  const output=run();
  assert.equal(output.provenance.schema,'ipe.product-ship-provenance.v1');
  assert.equal(output.provenance.contexts.length,18);
});
