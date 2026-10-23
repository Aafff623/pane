import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=readFileSync(new URL('../src/main.ts',import.meta.url),'utf8');
const styles=readFileSync(new URL('../src/styles.css',import.meta.url),'utf8');
test('navigation frames use explicit blue for overview and yellow for settings, independent of neutral theme accent',()=>{
 assert.match(styles,/#providers\s+\.navigation-focus-target\s*\{[^}]*--navigation-focus-color:\s*#3b82f6/);
 assert.match(styles,/#panel-scroll\s+\.navigation-focus-target\s*\{[^}]*--navigation-focus-color:\s*#eab308/);
 assert.match(styles,/outline:\s*2px solid var\(--navigation-focus-color/);
});
const ast=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const names=['scrollOverviewCard'];
const code=ts.transpile(ast.statements.filter(n=>ts.isFunctionDeclaration(n)&&names.includes(n.name?.text)).map(n=>n.getText(ast)).join('\n'),{target:ts.ScriptTarget.ES2020});
for(const zoom of [0.64,0.8,1,1.16,1.47,2.03]) {
 test(`overview jump centers in layout coordinates at zoom ${zoom}`,()=>{
  const scroller={scrollTop:200,clientHeight:400,offsetHeight:400,clientTop:0,scrollHeight:5000,getBoundingClientRect:()=>({top:100,height:400*zoom})};
  const card={getBoundingClientRect:()=>({top:100+800*zoom,height:200*zoom})};
  const ctx=vm.createContext({document:{querySelector:()=>scroller},overviewJumpBehavior:()=> 'auto',cancelAnimationFrame(){},overviewJumpFrame:0,currentUiScale:()=>zoom,Math});
  vm.runInContext(code,ctx);ctx.scrollOverviewCard(card);
  assert.equal(scroller.scrollTop,900);
 });
}

function functions(...names) {
 return ts.transpile(ast.statements.filter(n=>ts.isFunctionDeclaration(n)&&names.includes(n.name?.text)).map(n=>n.getText(ast)).join('\n'),{target:ts.ScriptTarget.ES2020});
}

for (const [top,height,expected] of [[-500,200,0],[8000,200,4600],[800,1200,920]]) {
 test(`navigation clamps scroll limits and keeps tall-card heading visible (${top}, ${height})`,()=>{
  const scroller={scrollTop:200,clientHeight:400,offsetHeight:400,clientTop:2,scrollHeight:5000,getBoundingClientRect:()=>({top:100,height:400})};
  const card={getBoundingClientRect:()=>({top:100+top+2,height})};
  let arrivals=0;
  const ctx=vm.createContext({document:{querySelector:()=>scroller},overviewJumpBehavior:()=> 'auto',cancelAnimationFrame(){},overviewJumpFrame:0,Math});
  vm.runInContext(code,ctx);ctx.scrollOverviewCard(card,scroller,()=>arrivals++);
  assert.equal(scroller.scrollTop,expected);assert.equal(arrivals,1);
 });
}

test('auth navigation shares one pending load instead of replacing highlighted DOM twice',async()=>{
 let calls=0,resolve;
 const pending=new Promise(r=>{resolve=r});
 const ctx=vm.createContext({authCenterLoading:null,loadAuthCenterNow(){calls++;return pending}});
 vm.runInContext(functions('loadAuthCenter'),ctx);
 const first=ctx.loadAuthCenter(),second=ctx.loadAuthCenter();
 assert.equal(first,second);assert.equal(calls,1);
 resolve();await first;
 await ctx.loadAuthCenter();assert.equal(calls,2);
});

test('navigation highlights on arrival, restores replaced targets, and cleans up after expiry',()=>{
 const classes=new Set();let mutations,expiry,arrival,disconnected=false;
 const target=()=>({classList:{add:c=>classes.add(c),remove:c=>classes.delete(c)}});
 let current=target();const scroller={classList:{add:c=>classes.add(c),remove:c=>classes.delete(c)}};
 const ctx=vm.createContext({navigationFocusCleanup(){},overviewJumpFrame:0,cancelAnimationFrame(){},
  document:{querySelector:s=>s==='#panel-scroll'?scroller:current,querySelectorAll:()=>[current]},
  window:{clearTimeout(){},setTimeout(fn){expiry=fn;return 1}},
  MutationObserver:class{constructor(fn){mutations=fn}observe(){}disconnect(){disconnected=true}},
  scrollOverviewCard(_target,_scroller,done){arrival=done}});
 vm.runInContext(functions('focusNavigationTarget'),ctx);
 ctx.focusNavigationTarget('[data-auth-group="codex"]','#panel-scroll');
 assert.ok(classes.has('navigation-focus-room'));assert.ok(!classes.has('navigation-focus-target'));
 arrival();assert.ok(classes.has('navigation-focus-target'));
 classes.delete('navigation-focus-target');current=target();mutations();assert.ok(classes.has('navigation-focus-target'));
 expiry();assert.ok(!classes.has('navigation-focus-target'));assert.equal(disconnected,true);
});

test('navigation removes the centering spacer once the highlight expires',()=>{
 const classes=new Set();let expiry,arrival;
 const scroller={classList:{add:c=>classes.add(c),remove:c=>classes.delete(c)}};
 const target=()=>({classList:{add:c=>classes.add(c),remove:c=>classes.delete(c)}});
 const ctx=vm.createContext({overviewJumpFrame:0,cancelAnimationFrame(){},
  document:{querySelector:s=>s==='#panel-scroll'?scroller:target(),querySelectorAll:()=>[]},
  window:{clearTimeout(){},setTimeout(fn){expiry=fn;return 1}},
  MutationObserver:class{observe(){}disconnect(){}},
  scrollOverviewCard(_target,_scroller,done){arrival=done}});
 vm.runInContext('let navigationFocusCleanup=()=>{};'+functions('focusNavigationTarget'),ctx);
 ctx.focusNavigationTarget('[data-auth-group="codex"]','#panel-scroll');
 assert.ok(classes.has('navigation-focus-room'),'spacer is needed while centering');
 arrival();expiry();
 assert.ok(!classes.has('navigation-focus-room'),'50vh spacer must not outlive the highlight');
});

test('reduced motion disables animated jumps',()=>{
 const ctx=vm.createContext({config:{jumpAnimation:'smooth'},reduceMotion:()=>true});
 vm.runInContext(functions('overviewJumpBehavior'),ctx);
 assert.equal(ctx.overviewJumpBehavior(),'auto');
});

test('leaving auth during its pending load cancels the old family jump',async()=>{
 let resolve,focuses=0;
 const pending=new Promise(r=>{resolve=r});
 const ctx=vm.createContext({panelNavigationRevision:1,loadAuthCenter:()=>pending,CSS:{escape:x=>x},focusNavigationTarget(){focuses++}});
 vm.runInContext(functions('focusAuthFamily'),ctx);
 const load=ctx.focusAuthFamily('codex');
 ctx.panelNavigationRevision++;resolve();await load;
 assert.equal(focuses,0);
 await ctx.focusAuthFamily('codex');assert.equal(focuses,1);
});
