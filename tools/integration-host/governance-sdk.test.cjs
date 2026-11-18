const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const path=require('node:path');
const root=path.resolve(__dirname,'../..');
const sdk=fs.readFileSync(path.join(root,'crates/ipe-integration/sdk/index.js'),'utf8');
const fixture=JSON.parse(fs.readFileSync(path.join(root,'testing/integration/hosted/governance-handoff.json'),'utf8'));
function evaluate(code) { return JSON.parse(vm.runInNewContext(sdk+'\nconst fixture='+JSON.stringify(fixture)+';\nJSON.stringify('+code+')',{}, {timeout:1000})); }
test('mixed v1 and governance v2 descriptions preserve receipts, source pins and candidate references',()=>{
 const out=evaluate(`(()=>{const observations=fixture.models.map(model=>ipe.observation(model.schema_version===2?ipe.governanceModel(model.entity.record,{scope:model.scope,claim:model.claim}):model,{provider:'fixture'},['native-read']));return ipe.exchange([],null,observations);})()`);
 assert.deepEqual(out.observations.map(o=>o.model),fixture.models);
 assert.equal(new Set(out.observations.filter(o=>o.model.schema_version===2).map(o=>o.model.entity.record.type)).size,7);
 for(const o of out.observations) {assert.deepEqual(o.receipts,['native-read']);assert.deepEqual(o.data,{provider:'fixture'});}
 const contact=out.observations.at(-1).model.entity.record;
 assert.equal(contact.owner.resolution,'candidate');assert.equal(contact.owner.source_revision,null);assert.equal(contact.endpoint,null);
});
test('governance builder copies descriptions and does not infer actor authority or fulfillment',()=>{
 const out=evaluate(`(()=>{const record=fixture.models[7].entity.record;const model=ipe.governanceModel(record,{scope:'release',claim:'declared'});record.title='changed';return {model,version:ipe.version};})()`);
 assert.equal(out.version,'0.7.0');assert.deepEqual(out.model,fixture.models[7]);
 assert.equal(out.model.authorization,undefined);assert.equal(out.model.publication,undefined);
});
test('governance builders reject missing identity/source and unsupported claim or envelope versions',()=>{
 for(const mutation of ["record.type='invented'","record.id.authority=''","record.source.revision=''","record.source=null","options.claim='inferred'","options.scope=''"]){
  assert.throws(()=>evaluate(`(()=>{const record=fixture.models[7].entity.record,options={scope:'release',claim:'declared'};${mutation};return ipe.governanceModel(record,options);})()`),/ipe_governance_invalid/);
 }
 assert.throws(()=>evaluate(`ipe.observation({schema_version:2,entity:{kind:'fact'}},null,['native'])`),/ipe_observation_invalid/);
 assert.throws(()=>evaluate(`ipe.observation({schema_version:3,entity:{kind:'governance'}},null,['native'])`),/ipe_observation_invalid/);
 assert.throws(()=>evaluate(`ipe.observation(fixture.models[7],null,[])`),/ipe_observation_invalid/);
});
