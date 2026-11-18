#!/usr/bin/env node
// Deterministic broker replay for synthetic ServiceNow provenance extraction.
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const here=__dirname;
const fixture=JSON.parse(fs.readFileSync(path.join(here,'provenance.json'),'utf8'));
const sdk=fs.readFileSync(path.resolve(here,'../../../../crates/ipe-integration/sdk/index.js'),'utf8');
const collector=fs.readFileSync(path.join(here,'provenance-collector.js'),'utf8');

function clone(value){return JSON.parse(JSON.stringify(value));}
function replayContext(control_id,options={}){
  const source=fixture.contexts.find(item=>item.control_id===control_id);
  if(!source)throw new Error('provenance_control_not_found');
  const context=clone(source);
  if(options.danglingPerson){
    const assignment=context.assignments.find(item=>item.role==='Reviewer')||context.assignments[0];
    context.people=context.people.filter(person=>person.id!==assignment.person_id);
    assignment.resolution='unresolved';
  }
  if(options.duplicatePerson&&context.people.length)context.people.push(clone(context.people[0]));
  for(const record of context.records){
    record.receipt='fixture:servicenow:'+record.sys_id;
    if(options.sourceReceipt)record.receipt=options.sourceReceipt;
  }
  let state=null,responses=[],observations=[],requests=[];
  const input={control_id,observed_at:fixture.as_of};
  for(let step=0;step<8;step++){
    const args=JSON.stringify([{input,operations:['read_control_context']},responses,state]);
    const wrapped=`${sdk}\n${collector}\nJSON.stringify(step(...JSON.parse(__ipe_args)))`;
    const result=JSON.parse(vm.runInNewContext(wrapped,{__ipe_args:args},{timeout:1000}));
    observations.push(...(result.observations||[]));
    if('outputs'in result)return {control_id,observations,requests,summary:result.outputs};
    state=result.state;
    responses=(result.requests||[]).map(request=>{
      requests.push(request);
      if(request.operation!=='read_control_context')throw new Error('unexpected_operation');
      if(options.denied)return {id:request.id,operation:request.operation,code:'provider_denied'};
      if(options.missing)return {id:request.id,operation:request.operation,data:[],receipt:'fixture:servicenow:empty',source_digest:fixture.source_revision};
      return {id:request.id,operation:request.operation,data:[{context}],receipt:'fixture:broker:context:'+context.records[0].sys_id,source_digest:fixture.source_revision,...(options.truncated?{truncated:true}:{})};
    });
  }
  throw new Error('provenance_step_limit');
}

function collectProvenance(options={}){
  const captures=fixture.contexts.map(context=>replayContext(context.control_id,options));
  const contexts=captures.map(capture=>{
    const emitted=capture.observations.find(observation=>observation.data&&observation.data.context_snapshot);
    if(emitted)return emitted.data.context_snapshot;
    return {control_id:capture.control_id,records:[],assignments:[],people:[],collection:{status:capture.summary.status,observed_at:capture.summary.observed_at||fixture.as_of,receipt:capture.summary.receipt||null,gaps:capture.summary.gaps||[]}};
  });
  return {schema:'ipe.product-ship-provenance.v1',simulated:true,contexts};
}

if(require.main===module){
  const outputIndex=process.argv.indexOf('--output');
  const target=outputIndex>=0?process.argv[outputIndex+1]:process.argv[2];
  if(outputIndex>=0&&!target)throw new Error('output_path_required');
  const output=JSON.stringify(collectProvenance(),null,2)+'\n';
  if(target&&target!=='-')fs.writeFileSync(path.resolve(target),output,{encoding:'utf8',flag:'w'});
  else process.stdout.write(output);
}
module.exports={replayContext,collectProvenance};
