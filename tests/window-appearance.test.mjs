import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
test('slider previews every percentage without moving its native window until release',()=>{
 const node=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true).statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='bindPopoverScaleSlider');
 const handlers={},calls=[],colors=[];
 const range={value:'100',style:{setProperty:(...args)=>colors.push(args)},addEventListener:(n,f)=>handlers[n]=f,removeEventListener(){}};
 const readout={textContent:'',style:{}};
 const ctx=vm.createContext({window:{clearTimeout(){}},normalizedWindowScale:x=>x,setPopoverScale:(...args)=>{calls.push(args);return Promise.resolve()},Math,Number});
 vm.runInContext(ts.transpile(node.getText(),{target:ts.ScriptTarget.ES2020}),ctx);
 ctx.bindPopoverScaleSlider(range,readout);
 for(let value=80;value<=175;value++){range.value=String(value);handlers.input();}
 assert.equal(calls.length,0);assert.equal(readout.textContent,'175%');
 assert.ok(new Set(colors.filter(x=>x[0]==='--scale-color').map(x=>x[1])).size>=96);
 handlers.change();assert.deepEqual(calls,[[1.75,true]]);
});
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
const names = ['normalizedWindowScale', 'configuredWindowScale', 'textScaleFactor', 'applyWindowScale', 'setPopoverScale', 'setUiTextSize', 'centerPopover', 'resetPopoverPosition', 'togglePopoverLock', 'currentUiScale', 'scaledUiRect', 'uiViewport', 'placeMenuAt'];
const code = ts.transpile(ast.statements.filter(n => ts.isFunctionDeclaration(n) && names.includes(n.name?.text)).map(n => n.getText(ast)).join('\n'), {target: ts.ScriptTarget.ES2020});
function harness(panel) {
  const calls = [];
  const classes = new Map();
  const ctx = vm.createContext({IS_PANEL_FORM: panel, window:{innerWidth:560,innerHeight:800}, POPOVER_LAYOUT_WIDTH: 560, config:{windowScale:1,uiTextSize:'small'}, Number, Math,getComputedStyle:()=>({getPropertyValue:()=> '1'}),
    document:{documentElement:{style:{setProperty(){}}},body:{classList:{toggle(name,value){classes.set(name,value);}},dataset:{}},querySelector(){return null;}},
    invoke:async (cmd,args)=>calls.push([cmd,args]),patchConfig:async p=>calls.push(['patch',p]),requestRender(){},applyPopoverDragRegion(){},syncAppearanceSettingsControls(){}});
  vm.runInContext('let popoverScaleRequest=null; let popoverScaleJob=null; let repositionPopoverSettings=null;\n'+code,ctx);return {ctx,calls,classes};
}
for (const panel of [false,true]) {
  test(`continuous scale persists from ${panel?'panel':'floating'} settings`,async()=>{
    const {ctx,calls}=harness(panel);await ctx.setPopoverScale(1.47);
    assert.equal(ctx.config.windowScale,1.47);
    assert.ok(calls.some(([cmd,a])=>cmd==='set_popover_scale' && a.factor===1.47));
    assert.ok(calls.some(([cmd,a])=>cmd==='patch' && a.windowScale===1.47));
  });
  test(`font persists independently from ${panel?'panel':'floating'} settings`,()=>{
    const {ctx,calls}=harness(panel);ctx.setUiTextSize('large');
    assert.equal(ctx.config.uiTextSize,'large');assert.equal(ctx.config.windowScale,1);
    assert.ok(calls.some(([cmd,a])=>cmd==='patch' && a.uiTextSize==='large'));
  });
}

test('overview columns follow discrete window scale independently of text size',async()=>{
  const {ctx,classes}=harness(false);
  for (const scale of [1,1.25,1.5,1.75]) {
    await ctx.setPopoverScale(scale);
    assert.equal(classes.get('popover-wide'),scale>=1.5);
    ctx.setUiTextSize('large');
    assert.equal(classes.get('popover-wide'),scale>=1.5);
    assert.equal(ctx.config.windowScale,scale);
  }
});

test('position reset uses shared native work-area centering and clears the stale anchor',async()=>{
  const {ctx,calls}=harness(false);
  ctx.config.popoverPos={x:201,y:196};
  await ctx.resetPopoverPosition();
  assert.ok(calls.some(([cmd])=>cmd==='center_popover'));
  assert.ok(calls.some(([cmd,args])=>cmd==='patch' && args.popoverPos===null));
});

test('locking returns manual placement to the stable centered default',async()=>{
  const {ctx,calls}=harness(false);
  ctx.config.popoverLock=false;
  await ctx.togglePopoverLock();
  assert.ok(calls.some(([cmd,args])=>cmd==='patch' && args.popoverLock===true && args.popoverPos===null));
  assert.ok(calls.some(([cmd])=>cmd==='center_popover'));
});

test('every integer percentage between 80 and 175 remains selectable',()=>{
 const {ctx}=harness(false);
 for(let pct=80;pct<=175;pct++) assert.equal(ctx.normalizedWindowScale(pct/100),pct/100);
});

test('rapid resize requests serialize and skip obsolete intermediate values',async()=>{
 const {ctx,calls}=harness(false);
 let release;
 let active=0,peak=0;
 ctx.invoke=async(cmd,args)=>{
   calls.push([cmd,args]);active++;peak=Math.max(peak,active);
   if(args.factor===0.8) await new Promise(resolve=>{release=resolve;});
   active--;return {w:500,h:700};
 };
 const first=ctx.setPopoverScale(0.8,false);
 for(let pct=81;pct<=174;pct++) ctx.setPopoverScale(pct/100,false);
 const last=ctx.setPopoverScale(1.75,true);
 release();await first;await last;
 assert.equal(peak,1);
 assert.deepEqual(calls.filter(([cmd])=>cmd==='set_popover_scale').map(([,a])=>a.factor),[0.8,1.75]);
 const writes=calls.filter(([cmd])=>cmd==='patch');
 assert.equal(writes.length,1);assert.equal(writes[0][1].windowScale,1.75);
});

test('preview does not persist or repaint the whole dashboard',async()=>{
 const {ctx,calls}=harness(false);
 ctx.requestRender=()=>{throw new Error('full dashboard repaint during drag');};
 for(const scale of [0.8,1.03,1.17]) await ctx.setPopoverScale(scale,false);
 assert.equal(calls.filter(([cmd])=>cmd==='patch').length,0);
});

test('DOMRect with x/y is an element anchor, not a cursor point',()=>{
 const {ctx}=harness(false);
 const menu={offsetWidth:252,offsetHeight:260,style:{}};
 ctx.placeMenuAt(menu,{x:480,y:30,left:480,top:30,right:520,bottom:60,width:40,height:30},{align:'right',prefer:'below',gap:8});
 assert.equal(menu.style.left,'268px');
 assert.equal(menu.style.top,'68px');
});
