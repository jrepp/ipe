/// <reference path="../../../crates/ipe-integration/sdk/index.d.ts" />
namespace GovernanceTypeFixture {
 const id:Ipe.GovernanceId={namespace:'fixture',authority:'release',local_id:'approver'};
 const source:Ipe.Source={definition:{namespace:'fixture',resource:'release',path:'governance.json'},revision:'r1',pointer:''};
 const principal:Ipe.GovernanceReference<'principal'>={kind:'principal',id,resolution:'resolved',source_revision:'r1'};
 const policy:Ipe.GovernanceReference<'policy'>={kind:'policy',id,resolution:'candidate',source_revision:null};
 const common={id,source};
 const records:Ipe.GovernanceRecord[]=[
  {...common,type:'principal',title:'Approver',principal_kind:'role'},
  {...common,type:'policy',title:'Approval policy',authority:principal,subjects:['operation'],requirements:[{id:'approval',description:'Require approval',specification:'Provider approval rule'}]},
  {...common,type:'contract',title:'Approval agreement',parties:[principal,principal],subjects:['operation'],requirements:[]},
  {...common,type:'control',title:'Gate',governing_rules:[policy],mechanism:'Provider gate'},
  {...common,type:'handoff',title:'Request approval',policy,from:principal,to:principal,trigger:'Pending',required_action:'Review',completion:'Approval recorded',continuation:'Request gate check'},
  {...common,type:'role_assignment',principal,role:principal,relationship:'approval_authority',valid_from:null,valid_until:null},
  {...common,type:'contact_method',owner:principal,channel:'ticket_queue',endpoint:null,availability:'unknown',purpose:'Approval',valid_until:null},
 ];
 const observations:Ipe.GovernanceObservation[]=records.map(record=>ipe.observation(ipe.governanceModel(record,{scope:'release',claim:'declared'}),null,['receipt']));
 const legacy:Ipe.Observation=ipe.observed(['receipt'],'release','declared').emit({kind:'definition',source,title:'Workflow',inputs:[],outputs:[]});
 ipe.exchange([],null,[legacy,...observations]);
 // @ts-expect-error A resolved reference must identify its source revision.
 const missingRevision:Ipe.GovernanceReference={kind:'principal',id,resolution:'resolved',source_revision:null};
 // @ts-expect-error A policy authority must reference a principal.
 const wrongAuthority:Ipe.GovernanceRecord={...common,type:'policy',title:'Policy',authority:policy,subjects:[],requirements:[]};
 // @ts-expect-error Governance descriptions do not permit inferred claims.
 ipe.governanceModel(records[0],{scope:'release',claim:'inferred'});
}
