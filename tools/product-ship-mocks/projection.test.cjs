const {test}=require('node:test');
const assert=require('node:assert/strict');
const {calculate,nominal,project,triangular}=require('./projection.js');
const close=(a,b)=>assert.ok(Math.abs(a-b)<1e-6);
test('quarterly executions and full persona jobs charge preparation once',()=>{
  const {input,result:r}=nominal();
  close(r.executions*91.25,4200000);close(r.workflows*365,1344000);
  assert.equal(r.preparations,500);
  // Weighted whole-job tokens, cached reads, cache writes, and long-context premium.
  const inputRate=(.37*10+.6*1+.03*12.5)*1.1;
  const jobCost=(2375000*inputRate+187500*50*1.05)/1e6;
  close(r.jobCost,jobCost);
  close(r.sourceCost,1344000*jobCost);
  close(r.savedCost,r.sourceCost*.08*.5-500*365*(12000*10+900*50)/1e6);
  close(r.earlyStopSavings,r.sourceCost*.96*.01*.5);
  close(r.combinedSavings,r.savedCost+r.earlyStopSavings);
  const daily=calculate({...input,periodDays:1});
  close(daily.executions/r.executions,91.25);assert.equal(daily.preparations,r.preparations);
  assert.ok(calculate({...input,policyContextShare:0}).savedCost<0);
  assert.ok(calculate({...input,longContextShare:0}).jobCost<r.jobCost);
  assert.ok(calculate({...input,cachedReadShare:.8}).jobCost<r.jobCost);
});
test('assumption windows reproduce without subtracting independent percentiles',()=>{
  const a=project({samples:1000}),b=project({samples:1000});assert.deepEqual(a,b);
  for(const v of Object.values(a.summary))assert.ok(v.p10<=v.p50&&v.p50<=v.p90);
  assert.equal(triangular(0,[1,2,3]),1);assert.equal(triangular(1,[1,2,3]),3);
});
test('headcount changes enterprise spend; zero prices and invalid inputs stay explicit',()=>{
  const small=project({salaryBudget:150000000,samples:1000}),large=project({salaryBudget:300000000,samples:1000});
  assert.equal(small.summary.savedCost.p50,large.summary.savedCost.p50);
  assert.equal(large.summary.computeBudget.p50,small.summary.computeBudget.p50*2);
  assert.equal(project({inputRate:0,outputRate:0,cachedRate:0,writeRate:0,samples:1000}).summary.savedCost.p50,0);
  assert.throws(()=>project({periodDays:0}));assert.throws(()=>project({inputRate:-1}));assert.throws(()=>project({samples:50001}));
});
