// Operator-installed example. This program owns traversal and interpretation.
// Pure declaration projection. GitHub I/O and source authorization stay in Rust.
function projectDeclaration(kind, d) {
  // Inputs already passed the SDK response boundary. Build local projections
  // without repeated JSON validation/copying; observation/exchange validate outputs.
  const select = (value, fields) => {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('invalid_projection');
    const result = Object.create(null);
    for (const field of fields) if (Object.hasOwn(value, field)) result[field] = value[field];
    return result;
  };
  const pick = (value, fields) => {
    const source = value || {};
    const result = select(source, fields);
    const residual = Object.keys(source).filter(k => !fields.includes(k) && !['jobs','steps','runs'].includes(k));
    if (residual.length) result.unmodeled = select(source, residual);
    return result;
  };
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
function executionKey(e) { return 'execution:'+JSON.stringify([e.namespace,e.resource,e.run,e.attempt,e.job,e.step]); }
function observation(entity,claim,scope,data,receipt) {
  return ipe.observation({schema_version:1,scope,claim,entity},data,[receipt]);
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
    return observation({kind:'fact',subject:executionKey(execution),predicate:data.predicate,value:data.value,authority:'github',execution},'provider_reported','execution:'+execution.resource,data,receipt);
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

function step(context, responses, state) {
  const input = context.input;
  const s = state || {queue:[{type:'resolve',kind:'workflow',repo:input.repository,ref:input.revision,path:input.workflow,depth:0}],seen:[],sequence:0,definitions:0,resolved:{},edges:[],pending:[],collections:[],gaps:[],phase:'dispatch'};
  const record = (kind,key,data,receipt) => capturedRecord(kind,key,data,receipt,input);
  const gap = (task,code) => {if(s.gaps.length<40)s.gaps.push({repository:task.repo,path:task.path,code});};
  const enqueue = task => {if(s.queue.length<64 && (task.depth||0)<=4)s.queue.push(task);else gap(task,'traversal_limit');};
  const elapsed = (a,b) => {const start=ipe.timestamp(a),end=ipe.timestamp(b);return start===null||end===null||end<start?null:end-start;};
  let observations=[];
  // Emit one job constraint per step. Large workflow projections must not spend
  // the same guest budget on declaration traversal and every job's obligations.
  if(s.constraint_work) {
    const work=s.constraint_work;
    const job=work.jobs.shift();
    const constraint=jobConstraint(job,work.source,{model:{permissions:work.permissions}},work.receipt);
    if(!work.jobs.length)delete s.constraint_work;
    return ipe.exchange([],s,[constraint]);
  }
  if(s.phase==='emit_invocations') {
    const batch=s.edges.splice(0,8).map(edge=>{const target=s.resolved[edge.target];const supplied=edge.supplied||{};const specs=target?.inputs||{};const names=[...new Set([...Object.keys(specs),...Object.keys(supplied)])];const bindings=names.map(name=>({name,origin:Object.hasOwn(supplied,name)?'caller':Object.hasOwn(specs[name]||{},'default')?'declared_default':'absent',value:Object.hasOwn(supplied,name)?supplied[name]:specs[name]?.default??null}));return observation({kind:'invocation',parent:edge.parent,callee:target?.source||null,requested:edge.requested,resolution:target?'resolved':'unknown',bindings},'declared',edge.scope,{reference:edge.requested},edge.receipt);});
    if(!s.edges.length)s.phase='finish';return ipe.exchange([],s,batch);
  }
  if(s.phase==='finish') return ipe.complete({collections:s.collections,coverage:'partial',definitions:s.definitions,gaps:s.gaps,publication:false,performance_scope:'bounded sample of workflow runs and jobs; not a population success rate'});
  if(s.phase==='response') {
    const task=s.task,r=ipe.response(responses,'r'+s.sequence);
    s.phase='dispatch';
    if(r.code) {
      if(task.type==='yaml'&&task.kind==='action'&&task.path.endsWith('action.yml')&&r.code==='provider_not_found') enqueue({...task,path:task.path.slice(0,-3)+'yaml',requested_path:task.requested_path||task.path});
      else {gap(task,r.code);if(task.kind==='workflow'&&task.depth===0&&s.definitions===0)throw new Error('root_unavailable');}
    } else {
      const rows=ipe.records(responses,'r'+s.sequence);
      if(r.truncated)gap(task,'source_sample_truncated');
      if(task.type==='resolve') {
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
          observations.push(record('outcome',key,data,r.receipt));
          observations.push(record('fact',key+'/conclusion',{subject:key,predicate:'external_job_conclusion',value:job.conclusion,definition_commit:task.ref,authority:'github_reported'},r.receipt));
        }
      }
    }
  }
  if(s.constraint_work)return ipe.exchange([],s,observations);
  if(s.pending.length){const batch=observations.concat(s.pending.splice(0,Math.max(0,16-observations.length)));return ipe.exchange([],s,batch);}
  while(s.queue.length) {
    const task=s.queue.shift(),key=task.type+':'+task.repo+'@'+task.ref+'/'+task.path+'/'+(task.run_id||'');
    if(s.seen.includes(key))continue;
    if(s.seen.length>=80){gap(task,'request_plan_limit');s.queue=[];break;}
    s.seen.push(key);s.task=task;s.sequence++;s.phase='response';
    let operation,params;
    if(task.type==='resolve'){operation='resolve_revision';params={repository:task.repo,revision:task.ref};}
    else if(task.type==='yaml'||task.type==='text'){operation=task.type==='yaml'?'read_definition':'read_source';params={repository:task.repo,revision:task.ref,path:task.path};}
    else if(task.type==='tree'){operation='list_tree';params={repository:task.repo,revision:task.ref};}
    else if(task.type==='runs'){operation='list_runs';params={repository:task.repo,revision:task.ref,workflow:task.path};}
    else{operation='list_jobs';params={repository:task.repo,run_id:task.run_id,attempt:task.attempt};}
    return ipe.exchange([ipe.call('r'+s.sequence,operation,params)],s,observations);
  }
  s.phase='emit_invocations';return ipe.exchange([],s,observations);
}
