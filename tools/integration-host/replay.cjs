// Deterministic broker replay. No network or credentials.
const vm=require('node:vm');
const sha='a'.repeat(40);
const input={repository:'fixture/repo',revision:sha,workflow:'.github/workflows/test.yml',allowed_repositories:['fixture/repo','fixture/action']};
function replay(sdk,source,{denied=false,truncated=false,governance=null}={}) {
 let state=null,responses=[],observations=[],requests=[],progress=[];
 for(let step=0;step<128;step++) {
  const args=JSON.stringify([{input,operations:governance?["list_environments","list_rulesets","get_ruleset"]:[]},responses,state]);
  const out=JSON.parse(vm.runInNewContext(sdk+'\n'+source+'\nJSON.stringify(step(...JSON.parse(args)))',{args},{timeout:1000}));
  observations.push(...(out.observations||[]));if(out.progress)progress.push(out.progress);
  if('outputs' in out)return {observations,requests,summary:out.outputs,progress};
  state=out.state;responses=out.requests.map(req=>{
   const {operation,params}=req;requests.push({operation,params});
   const base={id:req.id,operation};
   if(denied&&operation==='resolve_revision')return {...base,code:'provider_denied'};
   if(governance?.denied?.includes(operation))return {...base,code:'provider_denied'};
   if(operation==='read_definition'&&params.path==='action.yml')return {...base,code:'provider_not_found'};
   let data;
   if(operation==='resolve_revision')data=[{sha}];
   else if(operation==='read_definition')data=[{document:params.path==='action.yaml'?{name:'Fixture action',inputs:{thing:{default:'default'}},runs:{using:'node20',main:'index.js'}}:{name:'Test',jobs:{build:{name:'Build',steps:[{uses:'fixture/action@v1',with:{thing:'value'}},{uses:'fixture/action@v1'}]}}},locations:[]}];
   else if(operation==='read_source')data=[{text:'core.getInput("thing")'}];
   else if(operation==='list_environments')data=governance.environments||[];
   else if(operation==='list_rulesets')data=(governance.rulesets||[]).map(({id})=>({id}));
   else if(operation==='get_ruleset')data=[governance.rulesets.find(rule=>String(rule.id)===params.ruleset_id)];
   else if(operation==='list_tree')data=[];
   else if(operation==='list_runs')data=[{id:1,head_sha:sha,name:'Run',run_attempt:1,status:'completed',conclusion:'success',created_at:'2026-01-01T00:00:00Z',run_started_at:'2026-01-01T00:00:01Z'}];
   else if(operation==='list_jobs')data=[{id:2,head_sha:sha,name:'Build',status:'completed',conclusion:'success',started_at:'2026-01-01T00:00:01Z',completed_at:'2026-01-01T00:00:03Z',steps:[]}];
   else throw Error(operation);
   if(governance&&operation==='read_definition'&&params.path!== 'action.yaml')data[0].document.jobs.build.environment=governance.environmentReference||'production';
   return {...base,data,receipt:operation+JSON.stringify(params),source_digest:governance?.missingDigest&&operation==='list_environments'?null:governance?'fixture:'+operation:'fixture',truncated:(truncated&&operation==='list_tree')||!!governance?.truncated?.includes(operation)};
  });
 }
 throw Error('run_step_limit');
}
module.exports={replay};
