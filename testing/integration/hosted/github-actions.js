// Operator-installed example. This program owns traversal and interpretation.
// Pure declaration projection. GitHub I/O and source authorization stay in Rust.
function projectDeclaration(context, responses, state) {
  if (state === null) return ipe.requests([ipe.request('source', 'captured_document')], { requested: true });
  const source = ipe.records(responses, 'source')[0];
  const d = source.document;
  const pick = (value, fields) => {
    const source = value || {};
    const result = ipe.pick(source, fields);
    const residual = Object.keys(source).filter(k => !fields.includes(k) && !['jobs','steps','runs'].includes(k));
    if (residual.length) result.unmodeled = ipe.pick(source, residual);
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
  if (source.kind === 'workflow') {
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
    gaps.push({pointer:'',code:'repository_rules_environment_protection_and_run_results_not_captured'});
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
  return ipe.complete({kind:source.kind, model, references, expressions, gaps, coverage:'partial'});
}

function step(context, responses, state) {
  const input = context.input;
  const s = state || {queue:[{type:'resolve',kind:'workflow',repo:input.repository,ref:input.revision,path:input.workflow,depth:0}],seen:[],sequence:0,definitions:0,gaps:[],phase:'dispatch'};
  const record = (kind,key,data,receipt) => ({kind,key,data,receipts:[receipt]});
  const gap = (task,code) => {if(s.gaps.length<40)s.gaps.push({repository:task.repo,path:task.path,code});};
  const enqueue = task => {if(s.queue.length<64 && (task.depth||0)<=4)s.queue.push(task);else gap(task,'traversal_limit');};
  const elapsed = (a,b) => {const start=ipe.timestamp(a),end=ipe.timestamp(b);return start===null||end===null||end<start?null:end-start;};
  let observations=[];
  if(s.phase==='finish') return ipe.complete({coverage:'partial',definitions:s.definitions,gaps:s.gaps,publication:false,performance_scope:'bounded sample of workflow runs and jobs; not a population success rate'});
  if(s.phase==='response') {
    const task=s.task,r=ipe.response(responses,'r'+s.sequence);
    s.phase='dispatch';
    if(r.code) {
      if(task.type==='yaml'&&task.kind==='action'&&task.path.endsWith('action.yml')&&r.code==='provider_not_found') enqueue({...task,path:task.path.slice(0,-3)+'yaml'});
      else {gap(task,r.code);if(task.kind==='workflow'&&task.depth===0&&s.definitions===0)throw new Error('root_unavailable');}
    } else {
      const rows=ipe.records(responses,'r'+s.sequence);
      if(r.truncated)gap(task,'source_sample_truncated');
      if(task.type==='resolve') {
        const sha=rows[0]?.sha;
        if(typeof sha!=='string'||!/^[a-fA-F0-9]{40}$/.test(sha))throw new Error('invalid_revision');
        enqueue({...task,type:'yaml',ref:sha});
      } else if(task.type==='yaml') {
        const document=rows[0]?.document;
        if(!document || (task.kind==='workflow'&&!document.jobs) || (task.kind==='action'&&!document.runs))throw new Error('invalid_definition');
        const projection=projectDeclaration({},[{id:'source',receipt:r.receipt,data:[{kind:task.kind,document}]}],{}).outputs;
        const source={repository:task.repo,commit:task.ref,path:task.path,digest:r.source_digest,url:'https://github.com/'+task.repo+'/blob/'+task.ref+'/'+task.path};
        const requirements = task.kind==='workflow' ? projection.model.jobs.map(job=>({subject:job.id,operation:'github_actions.run_job',source_pointer:job.pointer,applicability:{dialect:'github_actions_expression',declared:job.if??null,evaluation:'unresolved'},dependencies:job.needs??[],requested_permissions:{value:job.permissions??projection.model.permissions??null,scope:job.permissions!==undefined?'job':projection.model.permissions!==undefined?'workflow':'unknown'},environment_reference:job.environment??null,unknowns:['effective_token_permissions','environment_protection','actor_authorization'],suggested_next_observation:'Read the exact workflow attempt and job outcome before assuming a prerequisite succeeded.'})) : Object.entries(projection.model.inputs||{}).map(([name,spec])=>({subject:name,operation:'github_actions.invoke_action',declared_required:spec.required??null,declared_default:spec.default??null,runtime_validation:'not_observed',source_pointer:'/inputs/'+name}));
        observations.push(record('contract',task.repo+'@'+task.ref+'/'+task.path,{kind:task.kind,source,projection,requirements,locations:rows[0].locations,interpretation:'declaration',external_enforcer:'github_actions'},r.receipt));
        s.definitions++;
        for(const reference of projection.references) {
          const uses=reference.uses;
          if(uses.startsWith('./')&&reference.kind==='workflow')enqueue({type:'yaml',repo:task.repo,ref:task.ref,path:uses.slice(2),kind:'workflow',depth:task.depth+1});
          else {
            const match=/^([^/]+\/[^/@]+)(\/[^@]+)?@([^@]+)$/.exec(uses);
            if(!match||uses.includes('${{')){gap(task,'unresolved_reference');continue;}
            const repo=match[1],ref=match[3];
            if(!input.allowed_repositories.includes(repo)){gap({repo,path:uses},'repository_outside_installation_scope');continue;}
            const path=reference.kind==='workflow'?(match[2]||'').slice(1):(match[2]||'').slice(1)+(match[2]?'/':'')+'action.yml';
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
  s.phase='finish';return ipe.exchange([],s,observations);
}
