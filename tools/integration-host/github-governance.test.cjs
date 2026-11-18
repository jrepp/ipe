const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {replay}=require('./replay.cjs'),root=path.resolve(__dirname,'../..');
const sdk=fs.readFileSync(path.join(root,'crates/ipe-integration/sdk/index.js'),'utf8'),collector=fs.readFileSync(path.join(root,'testing/integration/hosted/github-actions.js'),'utf8');
const environment={id:1,name:'production',protection_rules:[{type:'required_reviewers',prevent_self_review:true,reviewers:[{type:'User',reviewer:{id:7,type:'User',login:'reviewer',html_url:'https://github.com/reviewer'}},{type:'Team',reviewer:{id:8,name:'Release team',html_url:'https://github.com/orgs/example/teams/release'}}]}],deployment_branch_policy:{protected_branches:true,custom_branch_policies:false}};
const ruleset={id:2,name:'Main branch',target:'branch',enforcement:'active',conditions:{ref_name:{include:['refs/heads/main'],exclude:[]}},rules:[{type:'pull_request',parameters:{required_approving_review_count:1}}]};
const fixture={environments:[environment],rulesets:[ruleset]};
function governance(out){return out.observations.filter(o=>o.model.schema_version===2);}
test('GitHub capture retains environment rules, reviewer identity, handoff and separate ruleset scope',()=>{
 const out=replay(sdk,collector,{governance:fixture}),records=governance(out),entities=records.map(o=>o.model.entity.record);
 assert.ok(entities.some(r=>r.type==='handoff'));assert.ok(entities.some(r=>r.type==='role_assignment'));
 assert.equal(entities.find(r=>r.id.local_id==='user:7').principal_kind,'unknown');assert.equal(entities.find(r=>r.id.local_id==='team:8').principal_kind,'group');
 const policy=entities.find(r=>r.id.local_id==='environment:1');assert.equal(policy.subjects[0],'control:["github","fixture/repo","github_actions.run_job",".github/workflows/test.yml#build"]');assert.equal(policy.authority.resolution,'unknown');
 assert.deepEqual(entities.find(r=>r.id.local_id==='ruleset:2').subjects,['github:ruleset:fixture/repo:2:branch']);
 for(const o of records){assert.ok(o.receipts[0].startsWith(o.model.entity.record.source.definition.path==='api/environments'?'list_environments':'get_ruleset'));assert.ok(o.model.entity.record.source.revision.startsWith('fixture:'));}
 for(const record of entities)for(const value of Object.values(record)){const refs=Array.isArray(value)?value:[value];for(const ref of refs)if(ref?.resolution==='resolved')assert.ok(entities.some(target=>target.type===ref.kind&&JSON.stringify(target.id)===JSON.stringify(ref.id)&&target.source.revision===ref.source_revision));}
 assert.ok(out.observations.some(o=>o.model.entity.kind==='outcome'));assert.equal(out.summary.publication,false);
});
test('denied, missing revision and unresolved environment references remain explicit gaps',()=>{
 for(const [changes,code] of [[{denied:['list_environments']},'provider_denied'],[{missingDigest:true},'governance_source_revision_missing'],[{environmentReference:'${{ inputs.environment }}'},'environment_reference_unresolved'],[{truncated:['list_environments']},'source_sample_truncated']]){
  const out=replay(sdk,collector,{governance:{...fixture,...changes}});assert.ok(out.summary.gaps.some(g=>g.code===code));
  if(changes.environmentReference)assert.deepEqual(governance(out).find(o=>o.model.entity.record.id.local_id==='environment:1').model.entity.record.subjects,[]);
 }
});
test('sample limits, duplicate reviewers and unsafe profile URLs do not create false targets or contacts',()=>{
 const env=JSON.parse(JSON.stringify(environment));env.protection_rules[0].reviewers[0].reviewer.html_url='https://github.com/reviewer?token=bad';
 const out=replay(sdk,collector,{governance:{environments:[env,{...env,id:3,name:'secondary'},{...env,id:4}],rulesets:[]}}),entities=governance(out).map(o=>o.model.entity.record);
 assert.equal(entities.filter(r=>r.id.local_id==='user:7').length,1);assert.ok(out.summary.gaps.some(g=>g.code==='environment_sample_limit'));
 assert.equal(entities.find(r=>r.type==='contact_method'&&r.owner.id.local_id==='user:7').endpoint,null);
 assert.ok(entities.length<32);
});

test('a literal environment outside the captured sample remains explicitly unknown',()=>{
 const out=replay(sdk,collector,{governance:{...fixture,environmentReference:'uncaptured-production'}});
 assert.ok(out.summary.gaps.some(g=>g.code==='referenced_environment_not_captured'&&g.path==='environment:uncaptured-production'));
 const policy=governance(out).find(o=>o.model.entity.record.id.local_id==='environment:1').model.entity.record;
 assert.deepEqual(policy.subjects,[]);
});

test('governance construction stays within each emitted batch',()=>{
 const boundedSdk=sdk.replace('function governanceModel(record, options) {',`function governanceModel(record, options) {
   globalThis.constructedGovernance=(globalThis.constructedGovernance||0)+1;
   if(globalThis.constructedGovernance>4)throw new Error('too_many_governance_models_in_step');`);
 const out=replay(boundedSdk,collector,{governance:fixture});
 assert.equal(governance(out).length,13);
 assert.deepEqual(out,replay(sdk,collector,{governance:fixture}));
});
