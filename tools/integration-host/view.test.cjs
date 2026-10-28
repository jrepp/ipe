const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {JSDOM}=require('../../docs/node_modules/jsdom');
test('snapshot separates collector time from external duration and escapes provider text', async()=>{
 const html=fs.readFileSync(__dirname+'/index.html','utf8');
 const dom=new JSDOM(html,{runScripts:'outside-only',url:'http://localhost/'});
 try {
  const d=dom.window.document, data={list:{integrations:[{id:'sample'}]},history:{runs:[{id:1,trigger:'manual',status:'succeeded',elapsed_ms:100,metrics:{requests:1,retries:0}}],external_outcomes:[{key:'attempt/1',data:{name:'<img src=x onerror=alert(1)>',duration_ms:26000,conclusion:'success'}}]},records:{records:[{expired:true,record:{kind:'fact',key:'attempt/1',observed_at:1,run_id:1,data:{status:'success'},source_evidence:[{source_digest:'sha256:test'}]}}]}};
  const node=d.createElement('script');node.id='host-snapshot';node.type='application/json';node.textContent=JSON.stringify(data);d.body.append(node);
  dom.window.eval(d.querySelector('script').textContent);
  await new Promise(resolve=>setTimeout(resolve,0));
  assert.match(d.querySelector('#runs').textContent,/100 ms/);
  assert.match(d.querySelector('#outcomes').textContent,/26000 ms/);
  assert.match(d.querySelector('#outcomes').textContent,/Unknown/);
  assert.equal(d.querySelector('#outcomes img'),null);
  assert.match(d.querySelector('#records').textContent,/EXPIRED/);
  assert.match(d.querySelector('#records').textContent,/sha256:test/);
  assert.equal(d.querySelector('#run').disabled,true);
 } finally {dom.window.close();}
});
