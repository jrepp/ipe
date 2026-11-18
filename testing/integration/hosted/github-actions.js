// Operator-installed example. This program owns traversal and interpretation.
// Pure declaration projection. GitHub I/O and source authorization stay in Rust.
function projectDeclaration(kind, d) {
  const pick = (value, fields) => ipe.project(value || {}, fields, ['jobs','steps','runs']);
  const stepFields = ['id','name','uses','run','shell','if','with','env','working-directory','continue-on-error','timeout-minutes'];
  const steps = value => Array.isArray(value) ? value.map(s => pick(s, stepFields)) : [];
  const references = [];
  const gaps = [];
  const ref = (uses, pointer, kind, supplied) => {
    if (typeof uses === 'string') references.push({ uses, pointer, kind, supplied: supplied === undefined ? {} : supplied });
    else if (uses !== undefined) gaps.push({ pointer, code: 'invalid_uses' });
  };
  let model;
  if (kind === 'workflow') {
    model = pick(d, ['name','run-name','on','permissions','concurrency','env','defaults']);
    model.jobs = Object.entries(d.jobs || {}).map(([id, job]) => {
      const pointer = '/jobs/' + id.replace(/~/g,'~0').replace(/\//g,'~1');
      const out = pick(job, ['name','needs','if','runs-on','permissions','environment','strategy','concurrency','timeout-minutes','continue-on-error','outputs','uses','with','secrets','env','defaults','container','services']);
      out.id = id;
      out.pointer = pointer;
      out.steps = steps(job.steps);
      ref(job.uses, pointer + '/uses', 'workflow', job.with);
      out.steps.forEach((s,i) => ref(s.uses, pointer + '/steps/' + i + '/uses', 'action', s.with));
      return out;
    });

    // This YAML projection cannot establish API-managed governance. Separate reads do that.
    gaps.push({pointer:'',code:'repository_rules_and_environment_protection_not_captured'});
  } else {
    model = pick(d, ['name','description','author','inputs','outputs','branding']);
    model.runs = pick(d.runs, ['using','main','pre','pre-if','post','post-if','image','entrypoint','pre-entrypoint','post-entrypoint','args','env']);
    model.runs.steps = steps(d.runs && d.runs.steps);
    model.runs.steps.forEach((s,i) => ref(s.uses, '/runs/steps/' + i + '/uses', 'action', s.with));
    gaps.push({pointer:'/runs',code:'implementation_behavior_not_proven_by_metadata'});
  }
  const expressions = [];
  function walk(value, pointer) {
    if (typeof value === 'string' && (value.includes('${{') || pointer.endsWith('/if') || pointer.endsWith('-if'))) {
      expressions.push({pointer, expression:value, status:'unresolved'});
    } else if (value && typeof value === 'object') {
      Object.entries(value).forEach(([k,v]) => walk(v,pointer+'/'+k.replace(/~/g,'~0').replace(/\//g,'~1')));
    }
  }
  walk(d,'');
  return {kind, model, references, expressions, gaps, coverage:'partial'};
}

// Versioned records keep raw provider data separate from the typed model.
function sourceModel(source, pointer='') {
  return {definition:{namespace:'github',resource:source.repository,path:source.path},revision:source.commit,pointer};
}
function controlModel(repo,path,id) {
  return {namespace:'github',resource:repo,operation:'github_actions.run_job',local_id:path+'#'+id};
}
function executionModel(key) {
  const match=/^(.*)\/runs\/([^/]+)\/attempts\/(\d+)(?:\/jobs\/([^/]+))?$/.exec(key);
  if(!match)throw new Error('invalid_execution_identity');
  return {namespace:'github',resource:match[1],run:match[2],attempt:Number(match[3]),job:match[4]||null,step:null};
}

function observation(entity,claim,scope,data,receipt) {
  return ipe.observed([receipt],scope,claim).emit(entity,data);
}
function capturedRecord(kind,key,data,receipt,input) {
  if(kind==='contract') {
    const source=sourceModel(data.source),scope='definition:'+JSON.stringify(source.definition);
    if(data.kind==='implementation') return observation({kind:'evidence',source,method:'source_excerpt',description:'Captured source with lexical clues; runtime effects are not proven'},'inferred',scope,data,receipt);
    const declaration=data.projection.model;
    const inputs=Object.entries(declaration.inputs||{}).map(([name,spec])=>({name,required:typeof spec.required==='boolean'?spec.required:null,default:spec.default??null}));
    return observation({kind:'definition',source,title:declaration.name||data.source.path,inputs,outputs:Object.keys(declaration.outputs||{})},'declared',scope,data,receipt);
  }
  if(kind==='fact') {
    const execution=executionModel(data.subject);
    return observation({kind:'fact',subject:ipe.executionKey(execution),predicate:data.predicate,value:data.value,authority:'github',execution},'provider_reported','execution:'+execution.resource,data,receipt);
  }
  const execution=executionModel(key);
  const definition=sourceModel({repository:data.repository,path:input.workflow,commit:data.definition_commit||data.head_sha});
  const status=data.status==='completed'?'completed':data.status==='in_progress'?'in_progress':['queued','waiting','pending','requested'].includes(data.status)?'queued':'unknown';
  return observation({kind:'outcome',execution,definition,title:data.name||key,status,conclusion:status==='completed'?data.conclusion??null:null,duration_ms:data.duration_ms,started_at:data.started_at||data.run_started_at||null,completed_at:data.completed_at||null,declaration_candidates:(data.declaration_candidates||[]).map(id=>controlModel(data.repository,input.workflow,id)),correlation:data.declaration_candidates?.length?'candidate':'unknown'},'provider_reported','execution:'+execution.resource,data,receipt);
}
function jobConstraint(job,source,projection,receipt) {
  const control=controlModel(source.repository,source.path,job.id);
  const dependencies=(Array.isArray(job.needs)?job.needs:typeof job.needs==='string'?[job.needs]:[]).map(id=>controlModel(source.repository,source.path,id));
  const next={operation:'list_jobs',reason:'Observe the exact workflow attempt before assessing prerequisites'};
  const obligations=dependencies.map((dep,index)=>({id:'dependency_'+index,description:'Observe dependency '+dep.local_id+' under the provider condition semantics',requirement:{kind:'dependency',control:dep},next_observation:next}));
  obligations.push({id:'permissions',description:'Requested token permissions do not prove effective permissions',requirement:{kind:'permission',requested:job.permissions??projection.model.permissions??null},next_observation:null});
  if(job.environment)obligations.push({id:'environment',description:'Observe the referenced environment protection requirements',requirement:{kind:'environment',reference:job.environment},next_observation:null});
  obligations.push({id:'actor',description:'Establish authorization for the intended execution actor',requirement:{kind:'opaque',reason:'The discovery principal does not establish the execution actor authorization'},next_observation:null});
  const entity={kind:'constraint',control,source:sourceModel(source,job.pointer),title:job.name||job.id,actor_context:'intended_execution_actor_unobserved',applicability:{dialect:'github_actions_expression',expression:job.if??null,default_behavior:'provider_default_dependency_status'},obligations,dependencies,enforcer:'github_actions'};
  return observation(entity,'declared','workflow:'+source.repository+'/'+source.path,{source,job_id:job.id},receipt);
}

// REST governance descriptions retain their response digest and receipt. They do not
// establish approval, actor authorization, or an operating enforcement result.
function githubGovernance(task, response, subjects, gap) {
  if(typeof response.source_digest!=='string'||!response.source_digest){gap(task,'governance_source_revision_missing');return [];}
  const revision=response.source_digest, output=[],emitted=new Set();
  const specification=value=>{const text=JSON.stringify(value);if(text.length>512)gap(task,'governance_specification_truncated');return text.slice(0,512);};
  const path=task.type==='environments'?'api/environments':'api/rulesets/'+task.ruleset_id;
  const source=pointer=>({definition:{namespace:'github',resource:task.repo,path},revision,pointer});
  const id=local_id=>({namespace:'github',authority:task.repo,local_id});
  const ref=(kind,local_id,resolution='resolved')=>({kind,id:id(local_id),source_revision:resolution==='resolved'?revision:null,resolution});
  const emit=(record,pointer,data)=>{const key=record.type+':'+record.id.local_id;if(emitted.has(key))return;emitted.add(key);if(output.length>=96){gap(task,'governance_record_limit');return;}output.push(ipe.observation(ipe.governanceModel({...record,source:source(pointer)},{scope:'governance:'+task.repo,claim:'provider_reported'}),data,[response.receipt]));};
  const unknown=local=>ref('principal',local,'unknown');
  if(task.type==='environments') {
    if(response.data.length>2)gap(task,'environment_sample_limit');
    const capturedNames=new Set(response.data.slice(0,2).filter(env=>Number.isSafeInteger(env.id)&&typeof env.name==='string').map(env=>env.name));
    for(const name of Object.keys(subjects))if(!capturedNames.has(name))gap({repo:task.repo,path:'environment:'+name},'referenced_environment_not_captured');
    response.data.slice(0,2).forEach((env,index)=>{
      if(!Number.isSafeInteger(env.id)||typeof env.name!=='string'){gap(task,'invalid_environment');return;}
      const base='environment:'+env.id,pointer='/environments/'+index;
      const rules=Array.isArray(env.protection_rules)?env.protection_rules:[];
      const requirements=rules.slice(0,6).map((rule,i)=>({id:'rule_'+i,description:'GitHub environment '+String(rule.type||'unknown').replaceAll('_',' ')+' rule',specification:specification(rule)}));
      if(rules.length>6)gap(task,'environment_rule_limit');
      if(env.deployment_branch_policy)requirements.push({id:'branch_policy',description:'Deployment branch policy',specification:specification(env.deployment_branch_policy)});
      emit({type:'policy',id:id(base),title:'Environment '+env.name,authority:unknown(base+':authority'),subjects:Object.hasOwn(subjects,env.name)?subjects[env.name]:[],requirements},pointer,{id:env.id,name:env.name});
      emit({type:'control',id:id(base+':gate'),title:'Environment protection gate: '+env.name,governing_rules:[ref('policy',base)],mechanism:'GitHub reports environment protection configuration. Gate execution and requirement fulfillment are not observed.'},pointer,{rule_types:rules.map(rule=>rule.type).slice(0,6)});
      let reviewersCaptured=false;
      for(const [ruleIndex,rule] of rules.slice(0,6).entries()) {
        if(rule.type!=='required_reviewers')continue;
        if(reviewersCaptured){gap(task,'reviewer_rule_sample_limit');continue;}reviewersCaptured=true;
        const role=base+':reviewer_role:'+ruleIndex;
        emit({type:'principal',id:id(role),title:'Required reviewer for '+env.name,principal_kind:'role'},pointer+'/protection_rules/'+ruleIndex,{type:rule.type});
        const reviewers=Array.isArray(rule.reviewers)?rule.reviewers:[];
        if(reviewers.length>2)gap(task,'reviewer_sample_limit');
        reviewers.slice(0,2).forEach((entry,i)=>{
          const who=entry.reviewer||{},kind=entry.type==='Team'?'group':who.type==='Bot'?'service':who.type==='User'?'unknown':null;
          if(!kind||!Number.isSafeInteger(who.id)){gap(task,'reviewer_identity_unknown');return;}
          const person=(entry.type==='Team'?'team:':'user:')+who.id,loc=pointer+'/protection_rules/'+ruleIndex+'/reviewers/'+i;
          emit({type:'principal',id:id(person),title:who.login||who.name||person,principal_kind:kind},loc,{id:who.id,type:who.type??entry.type});
          emit({type:'role_assignment',id:id(base+':reviewer:'+person),principal:ref('principal',person),role:ref('principal',role),relationship:'approval_authority',valid_from:null,valid_until:null},loc,{reviewer_id:who.id,reviewer_type:entry.type});
          emit({type:'handoff',id:id(base+':handoff:'+person),title:'Request environment review',policy:ref('policy',base),from:unknown(base+':execution_actor'),to:ref('principal',person),trigger:'A deployment requests this environment',required_action:'Review the deployment under the provider rule',completion:'Provider records a qualifying review; not observed in this capture',continuation:'Provider evaluates remaining deployment protection rules'},loc,{prevent_self_review:rule.prevent_self_review??null});
          const endpoint=typeof who.html_url==='string'&&/^https:\/\/github\.com\/[^?#@\s]+$/.test(who.html_url)?who.html_url:null;
          emit({type:'contact_method',id:id(base+':contact:'+person),owner:ref('principal',person),channel:'provider_profile',endpoint,availability:endpoint?'available':'unknown',purpose:'Provider reviewer profile; delivery and availability are not verified',valid_until:null},loc,{html_url:endpoint});
        });
      }
    });
  } else {
    const rule=response.data[0];
    if(!rule||!Number.isSafeInteger(rule.id)){gap(task,'invalid_ruleset');return [];}
    const base='ruleset:'+rule.id,rules=Array.isArray(rule.rules)?rule.rules:[];
    if(rules.length>32)gap(task,'ruleset_rule_limit');
    // Provider subjects identify rule scope. They do not imply a workflow match.
    const ruleSubjects=['github:ruleset:'+task.repo+':'+rule.id+':'+String(rule.target||'unknown')];
    emit({type:'policy',id:id(base),title:rule.name||base,authority:unknown(base+':authority'),subjects:ruleSubjects,requirements:rules.slice(0,32).map((item,i)=>({id:'rule_'+i,description:'GitHub '+String(item.type||'unknown').replaceAll('_',' ')+' rule',specification:specification(item)}))},'',{id:rule.id,name:rule.name??null,target:rule.target??null});
    emit({type:'control',id:id(base+':mechanism'),title:'Repository ruleset mechanism',governing_rules:[ref('policy',base)],mechanism:'GitHub reports enforcement mode '+String(rule.enforcement||'unknown')+'. Scope conditions and bypass actors remain provider data; no execution compliance is established.'},'',{enforcement:rule.enforcement??null,conditions:specification(rule.conditions??null),bypass_actors:specification(rule.bypass_actors??[])});
  }
  return output;
}

function step(context, responses, state) {
  const input = context.input;
  const s = state || {work:null,definitions:0,resolved:{},edges:[],collections:[],phase:'dispatch'};
  const plan = ipe.tasks(s.work);
  s.work = plan.state;
  const record = (kind,key,data,receipt) => capturedRecord(kind,key,data,receipt,input);
  const gap = (task,code) => plan.gap({repository:task?.repo,path:task?.path},code);
  const enqueue = task => plan.enqueue(task,JSON.stringify([task.type,task.repo,task.ref,task.path,task.run_id||null,task.attempt||null]),task.depth||0);
  if(state===null)enqueue({type:'resolve',kind:'workflow',repo:input.repository,ref:input.revision,path:input.workflow,depth:0});
  const exchange = (requests,observations,phase,task=null) => ipe.exchange(requests,s,observations,plan.progress(phase,task));
  const elapsed = ipe.elapsed;
  let observations=[];
  if(s.governance?.length)return exchange([],ipe.emitBatch(s.governance,item=>item,8),'emit','governance');
  if(s.associations?.length) {
    const batch=ipe.emitBatch(s.associations,item=>observation(item.entity,'inferred','execution:'+input.repository,{},item.receipt),8);
    return exchange([],batch,'emit','association');
  }
  // Emit one job constraint per step. Large workflow projections must not spend
  // the same guest budget on declaration traversal and every job's obligations.
  if(s.constraint_work) {
    const work=s.constraint_work;
    const constraints=ipe.emitBatch(work.jobs,job=>jobConstraint(job,work.source,{model:{permissions:work.permissions}},work.receipt));
    if(!work.jobs.length)delete s.constraint_work;
    return exchange([],constraints,'emit','constraint');
  }
  if(s.phase==='emit_invocations') {
    const batch=ipe.emitBatch(s.edges,edge=>{const target=s.resolved[edge.target];const supplied=edge.supplied||{};const specs=target?.inputs||{};const names=[...new Set([...Object.keys(specs),...Object.keys(supplied)])];const bindings=names.map(name=>({name,origin:Object.hasOwn(supplied,name)?'caller':Object.hasOwn(specs[name]||{},'default')?'declared_default':'absent',value:Object.hasOwn(supplied,name)?supplied[name]:specs[name]?.default??null}));return observation({kind:'invocation',parent:edge.parent,callee:target?.source||null,requested:edge.requested,resolution:target?'resolved':'unknown',bindings},'declared',edge.scope,{reference:edge.requested},edge.receipt);},8);
    if(!s.edges.length)s.phase='finish';return exchange([],batch,'emit','invocation');
  }
  if(s.phase==='finish') return ipe.complete({collections:s.collections,coverage:'partial',definitions:s.definitions,gaps:plan.state.gaps.map(g=>({...g.task,code:g.code})),dropped_gaps:plan.state.droppedGaps,publication:false,performance_scope:'bounded sample of workflow runs and jobs; not a population success rate'});
  const received=plan.receive(responses);
  if(received) {
    const {task,result:r}=received;
    s.phase='dispatch';
    if(r.code) {
      if(task.type==='yaml'&&task.kind==='action'&&task.path.endsWith('action.yml')&&r.code==='provider_not_found') enqueue({...task,path:task.path.slice(0,-3)+'yaml',requested_path:task.requested_path||task.path});
      else {gap(task,r.code);if(task.kind==='workflow'&&task.depth===0&&s.definitions===0)throw new Error('root_unavailable');}
    } else {
      const rows=r.data;
      if(r.truncated)gap(task,'source_sample_truncated');
      if(task.type==='environments'||task.type==='ruleset') {
        s.governance=githubGovernance(task,r,s.environmentSubjects||{},gap);
        if(s.governance.length>96){s.governance=s.governance.slice(0,96);gap(task,'governance_record_limit');}
      } else if(task.type==='rulesets') {
        if(rows.length>6)gap(task,'ruleset_sample_limit');
        if(!(context.operations||[]).includes('get_ruleset'))gap({repo:task.repo,path:'get_ruleset'},'governance_not_granted');
        for(const row of rows.slice(0,6))if(Number.isSafeInteger(row.id)&&(context.operations||[]).includes('get_ruleset'))enqueue({...task,type:'ruleset',ruleset_id:String(row.id),path:'rulesets/'+row.id});
      } else if(task.type==='resolve') {
        const sha=rows[0]?.sha;
        if(typeof sha!=='string'||!/^[a-fA-F0-9]{40}$/.test(sha))throw new Error('invalid_revision');
        enqueue({...task,type:'yaml',ref:sha,requested_ref:task.ref});
      } else if(task.type==='yaml') {
        const document=rows[0]?.document;
        if(!document || (task.kind==='workflow'&&!document.jobs) || (task.kind==='action'&&!document.runs))throw new Error('invalid_definition');
        const projection=projectDeclaration(task.kind,document);
        const source={repository:task.repo,commit:task.ref,path:task.path,digest:r.source_digest,url:'https://github.com/'+task.repo+'/blob/'+task.ref+'/'+task.path};
        observations.push(record('contract',task.repo+'@'+task.ref+'/'+task.path,{kind:task.kind,source,projection,locations:rows[0].locations,interpretation:'declaration',external_enforcer:'github_actions'},r.receipt));
        if(task.kind==='workflow')s.collections.push({schema_version:1,scope:'workflow:'+task.repo+'/'+task.path,kind:'constraint',receipt:r.receipt});
        if(task.kind==='workflow'&&projection.model.jobs.length) {
          const fields=['id','pointer','name','needs','if','permissions','environment'];
          s.constraint_work={source,receipt:r.receipt,permissions:projection.model.permissions??null,jobs:projection.model.jobs.map(job=>Object.fromEntries(Object.entries(job).filter(([key])=>fields.includes(key))))};
        }
        s.resolved[task.repo+'@'+(task.requested_ref||task.ref)+'/'+(task.requested_path||task.path)]={source:sourceModel(source),inputs:Object.fromEntries(Object.entries(projection.model.inputs||{}).map(([name,spec])=>[name,Object.hasOwn(spec,'default')?{default:spec.default}:{}]))};
        s.definitions++;
        for(const reference of projection.references) {
          const uses=reference.uses;
          const edge={parent:sourceModel(source,reference.pointer),requested:uses,supplied:reference.supplied,receipt:r.receipt,scope:'workflow:'+task.repo+'/'+task.path,target:null};
          if(s.edges.length<64)s.edges.push(edge);else gap(task,'invocation_limit');
          if(uses.startsWith('./')&&reference.kind==='workflow'){edge.target=task.repo+'@'+task.ref+'/'+uses.slice(2);enqueue({type:'yaml',repo:task.repo,ref:task.ref,path:uses.slice(2),kind:'workflow',depth:task.depth+1});}
          else {
            const match=/^([^/]+\/[^/@]+)(\/[^@]+)?@([^@]+)$/.exec(uses);
            if(!match||uses.includes('${{')){gap(task,'unresolved_reference');continue;}
            const repo=match[1],ref=match[3];
            if(!input.allowed_repositories.includes(repo)){gap({repo,path:uses},'repository_outside_installation_scope');continue;}
            const path=reference.kind==='workflow'?(match[2]||'').slice(1):(match[2]||'').slice(1)+(match[2]?'/':'')+'action.yml';
            edge.target=repo+'@'+ref+'/'+path;
            enqueue({type:/^[a-fA-F0-9]{40}$/.test(ref)?'yaml':'resolve',kind:reference.kind,repo,ref,path,depth:task.depth+1});
          }
        }
        if(task.kind==='action') {
          const base=task.path.includes('/')?task.path.slice(0,task.path.lastIndexOf('/')+1):'';
          const run=document.runs;
          for(const key of ['pre','main','post'])if(typeof run[key]==='string')enqueue({type:'text',repo:task.repo,ref:task.ref,path:base+run[key].replace(/^\.\//,''),role:'declared_'+key,depth:task.depth});
          enqueue({type:'tree',repo:task.repo,ref:task.ref,path:base,depth:task.depth});
        } else if(task.depth===0) {
          s.environmentSubjects={};
          for(const job of projection.model.jobs){
            const env=typeof job.environment==='string'?job.environment:job.environment?.name;
            if(typeof env==='string'&&!env.includes('${{')){
              if(!Object.hasOwn(s.environmentSubjects,env))Object.defineProperty(s.environmentSubjects,env,{value:[],enumerable:true,writable:true,configurable:true});
              s.environmentSubjects[env].push(ipe.operationKey(controlModel(task.repo,task.path,job.id)));
            }
            else if(job.environment)gap(task,'environment_reference_unresolved');
          }
          for(const [type,operation] of [['environments','list_environments'],['rulesets','list_rulesets']]){
            if((context.operations||[]).includes(operation))enqueue({type,repo:task.repo,path:type,depth:0});
            else gap({repo:task.repo,path:operation},'governance_not_granted');
          }
          s.jobDeclarations=projection.model.jobs.map(job=>({id:job.id,name:job.name||job.id}));
          enqueue({type:'runs',repo:task.repo,ref:task.ref,path:task.path.split('/').at(-1),depth:0});
        }
      } else if(task.type==='tree') {
        const candidates=rows.filter(row=>row.type==='blob'&&typeof row.path==='string'&&row.path.startsWith(task.path+'src/')&&/\.(ts|js)$/.test(row.path)&&!row.path.includes('.test.'));
        candidates.sort((a,b)=>Number(!/\/(main|index|setup-python|input-helper)\.(ts|js)$/.test(a.path))-Number(!/\/(main|index|setup-python|input-helper)\.(ts|js)$/.test(b.path))||a.path.localeCompare(b.path));
        for(const row of candidates.slice(0,2))enqueue({...task,type:'text',path:row.path,role:'author_source_clue'});
        if(candidates.length>2)gap(task,'author_source_sample');
      } else if(task.type==='text') {
        const text=rows[0]?.text||'';
        const clues=[];text.split('\n').forEach((line,i)=>{if(clues.length<24&&['getInput(','setOutput(','getOctokit(','exec(','fetch(','process.env','saveState('].some(marker=>line.includes(marker)))clues.push({line:i+1,text:line.slice(0,300),interpretation:'lexical_clue_only'});});
        observations.push(record('contract','implementation:'+task.repo+'@'+task.ref+'/'+task.path,{kind:'implementation',source:{repository:task.repo,commit:task.ref,path:task.path,digest:r.source_digest},role:task.role,clues,excerpt:text,coverage:r.truncated?'partial':'captured_text_only'},r.receipt));
      } else if(task.type==='runs') {
        for(const run of rows.slice(0,3)) {
          if(String(run.head_sha)!==task.ref){gap(task,'run_commit_mismatch');continue;}
          const key=task.repo+'/runs/'+run.id+'/attempts/'+run.run_attempt;
          const data={...run,kind:'workflow_run',repository:task.repo,definition_commit:task.ref,created_to_started_ms:elapsed(run.created_at,run.run_started_at),duration_ms:null,duration_reason:'workflow API has no authoritative completion timestamp',measurement_authority:'github_reported'};
          observations.push(record('outcome',key,data,r.receipt));
          observations.push(record('fact',key+'/status',{subject:key,predicate:'external_run_status',value:{status:run.status,conclusion:run.conclusion},definition_commit:task.ref,authority:'github_reported'},r.receipt));
          enqueue({type:'jobs',repo:task.repo,ref:task.ref,path:task.path,run_id:String(run.id),attempt:String(run.run_attempt),depth:0});
        }
      } else if(task.type==='jobs') {
        for(const job of rows.slice(0,6)) {
          if(job.head_sha && job.head_sha!==task.ref){gap(task,'job_commit_mismatch');continue;}
          const key=task.repo+'/runs/'+task.run_id+'/attempts/'+task.attempt+'/jobs/'+job.id;
          const data={id:job.id,name:job.name,status:job.status,conclusion:job.conclusion,started_at:job.started_at,completed_at:job.completed_at,head_sha:task.ref,workflow_run_id:task.run_id,run_attempt:task.attempt,declared_job_id:null,declaration_candidates:(s.jobDeclarations||[]).filter(d=>d.name===job.name).map(d=>d.id),correlation:'static_name_only',workflow_path:'.github/workflows/'+task.path,duration_ms:elapsed(job.started_at,job.completed_at),steps:(job.steps||[]).map(x=>({name:x.name,number:x.number,status:x.status,conclusion:x.conclusion,duration_ms:elapsed(x.started_at,x.completed_at)})),measurement_authority:'github_reported',kind:'job',repository:task.repo};
          for(const id of data.declaration_candidates) {
            const entity={kind:'association',subject:ipe.controlKey(controlModel(task.repo,input.workflow,id)),predicate:'ipe.control_execution',target:ipe.executionKey(executionModel(key)),context:{execution:ipe.executionKey(executionModel(task.repo+'/runs/'+task.run_id+'/attempts/'+task.attempt)),revision:task.ref},authority:'github',resolution:'candidate'};
            s.associations=s.associations||[];
            if(s.associations.length<64)s.associations.push({entity,receipt:r.receipt});else gap(task,'association_limit');
          }
          observations.push(record('outcome',key,data,r.receipt));
          observations.push(record('fact',key+'/conclusion' ,{subject:key,predicate:'external_job_conclusion',value:job.conclusion,definition_commit:task.ref,authority:'github_reported'},r.receipt));
        }
      }
    }
  }
  if(s.governance?.length||s.constraint_work||s.associations?.length)return exchange([],observations,'emit',s.governance?.length?'governance':s.constraint_work?'constraint':'association');
  const task=plan.next();
  if(task) {
    let operation,params;
    if(task.type==='environments'){operation='list_environments';params={repository:task.repo};}
    else if(task.type==='rulesets'){operation='list_rulesets';params={repository:task.repo};}
    else if(task.type==='ruleset'){operation='get_ruleset';params={repository:task.repo,ruleset_id:task.ruleset_id};}
    else if(task.type==='resolve'){operation='resolve_revision';params={repository:task.repo,revision:task.ref};}
    else if(task.type==='yaml'||task.type==='text'){operation=task.type==='yaml'?'read_definition':'read_source';params={repository:task.repo,revision:task.ref,path:task.path};}
    else if(task.type==='tree'){operation='list_tree';params={repository:task.repo,revision:task.ref};}
    else if(task.type==='runs'){operation='list_runs';params={repository:task.repo,revision:task.ref,workflow:task.path};}
    else{operation='list_jobs';params={repository:task.repo,run_id:task.run_id,attempt:task.attempt};}
    return exchange([plan.request(task,operation,params)],observations,'dispatch',task.type);
  }
  s.phase='emit_invocations';return exchange([],observations,'emit','invocation');
}
