(function(root,factory){const api=factory();if(typeof module==='object'&&module.exports)module.exports=api;else root.IpeProjection=api;})(typeof globalThis==='object'?globalThis:this,function(){
  'use strict';
  const pricing={model:'GPT-6 Astra',tier:'Standard, short context',input:10,cached:1,write:12.5,output:50,checked:'2026-11-03',url:'https://developers.openai.com/api/docs/models/gpt-6-astra'};
  const personas=[
    {id:'engineering',label:'Engineering change',share:.5,input:[500000,3000000,12000000],output:[30000,200000,1000000],seconds:[600,1800,7200]},
    {id:'product',label:'Product plan',share:.25,input:[250000,1500000,6000000],output:[25000,150000,600000],seconds:[900,3600,14400]},
    {id:'design',label:'Design iteration',share:.25,input:[300000,2000000,8000000],output:[40000,200000,800000],seconds:[600,2400,10800]},
  ];
  const assumptions={policies:500,consumers:[3000,3500,4000],executionsPerConsumer:[300,1200,3000],agentWorkflowsPerExecution:[.02,.08,.20],
    preparationsPerPolicyPerDay:[.1,1,4],cachedReadShare:[.2,.6,.9],cacheWriteShare:[0,.03,.1],longContextShare:[0,.1,.4],policyContextShare:[.02,.08,.20],contextReduction:[.2,.5,.75],
    earlyStopShare:[0,.01,.05],remainingWorkShare:[.2,.5,.8],jobsPerEmployeePerDay:[5,25,80],
    prepInput:12000,prepOutput:900,prepSeconds:18,daysPerYear:365,salaryPerEmployee:150000};
  function random(seed){let state=seed>>>0;return ()=>{state=(1664525*state+1013904223)>>>0;return state/4294967296;};}
  function triangular(u,[low,mode,high]){const split=(mode-low)/(high-low);return u<split?low+Math.sqrt(u*(high-low)*(mode-low)):high-Math.sqrt((1-u)*(high-low)*(high-mode));}
  function rates(options={}){return {input:options.inputRate??pricing.input,output:options.outputRate??pricing.output,cached:options.cachedRate??pricing.cached,write:options.writeRate??pricing.write};}
  function calculate(input,price=pricing){
    const executions=input.consumers*input.executionsPerConsumer/input.periodDays,workflows=executions*input.agentWorkflowsPerExecution;
    const preparations=assumptions.policies*input.preparationsPerPolicyPerDay;
    const cached=Math.min(input.cachedReadShare,1-input.cacheWriteShare),uncached=1-cached-input.cacheWriteShare;
    const inputRate=(uncached*price.input+cached*price.cached+input.cacheWriteShare*price.write)*(1+input.longContextShare);
    const outputRate=price.output*(1+.5*input.longContextShare);
    const roleRows=personas.map(p=>{const tokens=input.personas[p.id];return {...p,...tokens,cost:(tokens.input*inputRate+tokens.output*outputRate)/1e6};});
    const average=key=>roleRows.reduce((sum,r)=>sum+r[key]*r.share,0);
    const jobCost=average('cost'),jobTokens=average('input')+average('output'),jobSeconds=average('seconds');
    const reduction=input.policyContextShare*input.contextReduction;
    const prepCost=preparations*(assumptions.prepInput*price.input+assumptions.prepOutput*price.output)/1e6;
    const sourceCost=workflows*jobCost*365,modeledCost=(workflows*jobCost*(1-reduction)+prepCost)*365;
    const savedCost=sourceCost-modeledCost;
    const earlyStopSavings=workflows*jobCost*(1-reduction)*input.earlyStopShare*input.remainingWorkShare*365;
    const enterpriseJobs=input.salaryBudget/assumptions.salaryPerEmployee*input.jobsPerEmployeePerDay;
    const computeBudget=enterpriseJobs*jobCost*365;
    return {executions,workflows,preparations,jobCost,rawTokens:workflows*jobTokens,modeledTokens:workflows*jobTokens*(1-reduction)+preparations*(assumptions.prepInput+assumptions.prepOutput),
      savedTokens:workflows*jobTokens*reduction-preparations*(assumptions.prepInput+assumptions.prepOutput),savedHours:(workflows*jobSeconds*reduction-preparations*assumptions.prepSeconds)/3600,
      sourceCost,modeledCost,savedCost,earlyStopSavings,combinedSavings:savedCost+earlyStopSavings,computeBudget,savingsShare:computeBudget?savedCost/computeBudget*100:0,
      exceedsSalary:computeBudget>input.salaryBudget?1:0,budgetExceeded:workflows>enterpriseJobs?1:0};
  }
  function nominal(options={}){const input={salaryBudget:options.salaryBudget??150000000,periodDays:options.periodDays??91.25,personas:{}};for(const[k,v]of Object.entries(assumptions))if(Array.isArray(v))input[k]=v[1];for(const p of personas)input.personas[p.id]={input:p.input[1],output:p.output[1],seconds:p.seconds[1]};return {input,result:calculate(input,rates(options))};}
  function project({salaryBudget=150000000,periodDays=91.25,samples=20000,seed=20261103,...options}={}){
    const price=rates(options);
    if(!Number.isFinite(salaryBudget)||salaryBudget<=0||![1,7,30,91.25].includes(periodDays)||!Number.isInteger(samples)||samples<100||samples>50000||Object.values(price).some(v=>!Number.isFinite(v)||v<0))throw new Error('Invalid projection inputs');
    const rand=random(seed),columns={};
    for(let i=0;i<samples;i++){
      const input={salaryBudget,periodDays,personas:{}};
      for(const[k,v]of Object.entries(assumptions))if(Array.isArray(v))input[k]=triangular(rand(),v);
      for(const p of personas)input.personas[p.id]={input:triangular(rand(),p.input),output:triangular(rand(),p.output),seconds:triangular(rand(),p.seconds)};
      const row=calculate(input,price);for(const[k,v]of Object.entries(row))(columns[k]??=[]).push(v);
    }
    const summary={};for(const[k,v]of Object.entries(columns)){v.sort((a,b)=>a-b);summary[k]={p10:v[Math.floor((samples-1)*.1)],p50:v[Math.floor((samples-1)*.5)],p90:v[Math.floor((samples-1)*.9)]};}
    const fraction=key=>columns[key].reduce((a,b)=>a+b,0)/samples;
    return {summary,samples,seed,salaryBudget,periodDays,overBudgetFraction:fraction('budgetExceeded'),exceedsSalaryFraction:fraction('exceedsSalary'),negativeSavingsFraction:columns.savedCost.filter(v=>v<0).length/samples};
  }
  return {assumptions,personas,pricing,calculate,nominal,project,triangular,rates};
});
