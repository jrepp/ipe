const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {JSDOM}=require('../../docs/node_modules/jsdom');
const root={id:0,kind:'workflow',source:{repository:'org/repo',path:'.github/workflows/test.yml',commit:'a'.repeat(40),digest:'sha256:abc',url:'https://github.com/org/repo/blob/'+ 'a'.repeat(40) +'/.github/workflows/test.yml'},requested_ref:'main',ref_was_pinned:false,source_text:'on: push',locations:{'/jobs/build/if':{line:4}},projection:{model:{name:'Build',jobs:[]},expressions:[{pointer:'/jobs/build/if',expression:'${{ github.ref }}'}],gaps:[]},implementation:[]};
const action={...root,id:1,kind:'action',source:{...root.source,repository:'actions/test',path:'action.yml'},projection:{model:{name:'<img src=x onerror=alert(1)>',inputs:{token:{default:'${{ github.token }}'}}},expressions:[],gaps:[]}};
const report={schema:'ipe.github-contract.v1',observed_at_unix:1,scope:{repository:'org/repo',workflow:root.source.path},documents:[root,action],edges:[{from:0,to:1,pointer:'/jobs/build/steps/0/uses',bindings:{inputs:[{input:'token',origin:'declared_default',value:'${{ github.token }}'}],undeclared_caller_inputs:[]}}],gaps:[],limitations:['No runtime outcomes.']};
async function page(data){
 const dom=new JSDOM(fs.readFileSync(__dirname+'/index.html','utf8'),{url:'http://localhost/',runScripts:'outside-only'});
 const el=dom.window.document.createElement('script');el.type='application/json';el.id='capture-data';el.textContent=JSON.stringify(data);dom.window.document.body.append(el);
 dom.window.eval(fs.readFileSync(__dirname+'/view.js','utf8'));await new Promise(r=>setTimeout(r,0));return dom;
}
test('report links declarations, input defaults and exact source locations without executing source HTML',async()=>{
 const dom=await page(report),d=dom.window.document;
 try{
  assert.match(d.querySelector('#summary').textContent,/2captured definitions/);
  assert.ok(d.querySelector('#detail a[href$="#L4"]'));
  d.querySelector('[data-document="1"]').click();
  assert.match(d.querySelector('#detail').textContent,/declared_default/);
  assert.match(d.querySelector('#detail').textContent,/<img src=x onerror=alert\(1\)>/);
  assert.equal(d.querySelector('#detail img'),null);
  const search=d.querySelector('#search');search.value='missing';search.dispatchEvent(new dom.window.Event('input'));
  assert.match(d.querySelector('#sources').textContent,/No sources match/);
 }finally{dom.window.close();}
});
test('unsupported capture shows an error instead of a success view',async()=>{
 const dom=await page({schema:'unknown'});
 try{assert.match(dom.window.document.querySelector('#summary').textContent,/could not be loaded/);}finally{dom.window.close();}
});
