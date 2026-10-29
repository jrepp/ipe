// Pure declaration projection. GitHub I/O and source authorization stay in Rust.
function step(context, responses, state) {
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
