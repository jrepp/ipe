'use strict';

// Daily summary samples for chart design. These are not captured agent runs.
function createOverviewHistory(model, revision) {
  const runs=[18,22,20,24,26,28,30], errors=[1,1,0,1,0,0,0];
  const allowed=[8,11,10,14,17,20,22], denied=[3,4,4,4,4,4,5];
  const seconds=[62,54,57,43,39,34,31];
  return {
    schema:'ipe.policy-overview-history.v1', measured:false,
    scope:{release_id:model.release.id,artifact:model.release.artifact,environment:model.release.environment,model_revision:revision,as_of:model.as_of},
    basis:'Seven daily summary samples. Each completed run has a paired direct-source comparison for the same checks and outcome. Errors are excluded from both cost paths. Each modeled run includes preparation once. No live usage was collected.',
    days:runs.map((count,index)=>{
      const completed=count-errors[index];
      return {
        date:new Date(Date.parse(model.as_of)-(6-index)*86400000).toISOString().slice(0,10),
        runs:count,errors:errors[index],allow:allowed[index],deny:denied[index],unknown:completed-allowed[index]-denied[index],
        paired_runs:completed,
        baseline_elapsed_ms:completed*180000,modeled_elapsed_ms:completed*seconds[index]*1000,
        baseline_input_tokens:completed*78000,baseline_output_tokens:completed*10800,
        modeled_input_tokens:completed*(54000-index*1800),modeled_output_tokens:completed*5550,
      };
    }),
  };
}
module.exports={createOverviewHistory};
