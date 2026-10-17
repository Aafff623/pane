import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildPixelGrid, buildPixelClipFrames, initThemeSwap } from './public/theme-swap.js';

test('PixelSwap grid caps pixel count and covers the complete viewport', () => {
  for (const [width, height] of [[390,844],[1440,900],[3840,2160],[8,8000]]) {
    const grid = buildPixelGrid(width,height,16);
    assert.ok(grid.pixels.length <= 220);
    assert.ok(Math.min(...grid.pixels.map(p=>p.left)) <= 0);
    assert.ok(Math.min(...grid.pixels.map(p=>p.top)) <= 0);
    assert.ok(Math.max(...grid.pixels.map(p=>p.left+grid.size)) >= width);
    assert.ok(Math.max(...grid.pixels.map(p=>p.top+grid.size)) >= height);
    assert.deepEqual(grid,buildPixelGrid(width,height,16),'stable random ordering across redraws');
  }
});

test('PixelSwap begins with no incoming page and finishes with overlapping full tiles', () => {
  const grid=buildPixelGrid(1440,900), frames=buildPixelClipFrames(grid);
  assert.equal(frames[0].offset,0);
  assert.equal(frames.at(-1).offset,1);
  for(const frame of frames) assert.ok(!/NaN|Infinity/.test(frame.clipPath));
  assert.equal([...frames[0].clipPath.matchAll(/h0\.00v0\.00/g)].length,grid.pixels.length);
  assert.equal([...frames.at(-1).clipPath.matchAll(new RegExp(`h${(grid.size+1).toFixed(2)}v`,'g'))].length,grid.pixels.length);
});

function fixture({ reduced=false, supported=false, broken=false, effectFails=false, initialTheme='dark' }={}) {
  const root={dataset:{theme:initialTheme}};
  const button=new EventTarget(), win=new EventTarget(), doc=new EventTarget(), preference=new EventTarget();
  const attributes=new Map(), icon={}, stored=new Map(), commits=[], animations=[], metadata=new Map();
  let finishTransition=()=>{};
  button.querySelector=()=>icon;
  button.getBoundingClientRect=()=>({left:10,right:50,top:10,bottom:50});
  button.setAttribute=(key,value)=>attributes.set(key,value);
  button.removeAttribute=key=>attributes.delete(key);
  preference.matches=reduced;
  win.matchMedia=()=>preference;
  win.innerWidth=1440; win.innerHeight=900;
  win.localStorage={setItem:(key,value)=>stored.set(key,value)};
  doc.hidden=false;
  doc.querySelector=()=>({setAttribute:(key,value)=>metadata.set(key,value)});
  if(supported) {
    root.animate=()=>{
      let resolve;
      const finished=new Promise(r=>{resolve=r;});
      const animation={finished,cancel:()=>{resolve();},finish:()=>{resolve();}};
      animations.push(animation);
      finished.then(()=>finishTransition());
      return animation;
    };
    doc.startViewTransition=update=>{
      if(broken) throw new Error('Snapshot unavailable');
      const updateCallbackDone=Promise.resolve().then(update);
      const finished=new Promise((resolve,reject)=>{finishTransition=resolve;updateCallbackDone.catch(reject);});
      return {ready:updateCallbackDone,updateCallbackDone,finished,skipTransition:()=>finishTransition()};
    };
  }
  const controller=initThemeSwap({root,button,doc,win,syncEffects:async()=>{commits.push(root.dataset.theme);if(effectFails)throw new Error('Effect unavailable');}});
  const click=()=>button.dispatchEvent(new Event('click'));
  return {root,button,win,doc,icon,attributes,stored,commits,animations,metadata,controller,click};
}
const tick=()=>new Promise(resolve=>setImmediate(resolve));

test('restoring light mode synchronises the button and browser chrome before another click',()=>{
  const env=fixture({initialTheme:'light'});
  assert.equal(env.icon.src,'/icons/moon.svg');
  assert.equal(env.attributes.get('aria-pressed'),'true');
  assert.equal(env.metadata.get('content'),'#f4f5f3');
  assert.equal(env.commits.length,0,'initialisation does not toggle or animate the page');
  env.controller.destroy();
});

test('theme fallback persists the setting and synchronises effects without an overlay',async()=>{
  const env=fixture(); env.click(); await tick();
  assert.equal(env.root.dataset.theme,'light');
  assert.equal(env.stored.get('pane-site-theme'),'light');
  assert.equal(env.icon.src,'/icons/moon.svg');
  assert.deepEqual(env.commits,['light']);
  assert.equal(env.root.dataset.themeTransition,undefined);
  env.controller.destroy();
});

test('reduced motion bypasses pixel animation even when snapshots are supported',async()=>{
  const env=fixture({supported:true,reduced:true}); env.click(); await tick();
  assert.equal(env.root.dataset.theme,'light');
  assert.equal(env.animations.length,0);
  env.controller.destroy();
});

test('rapid theme clicks settle to the latest intent and remove transition state',async()=>{
  const env=fixture({supported:true}); env.click(); await tick();
  assert.equal(env.animations.length,1);
  env.click(); await tick();
  assert.equal(env.root.dataset.theme,'dark');
  env.animations.at(-1).finish(); await tick();
  assert.equal(env.stored.get('pane-site-theme'),'dark');
  assert.equal(env.root.dataset.themeTransition,undefined);
  assert.equal(env.attributes.has('aria-busy'),false);
  env.controller.destroy();
});

test('a click on the captured theme button still reaches the toggle while root is hit-tested as HTML',async()=>{
  const env=fixture({supported:true}); env.click(); await tick();
  const event=new Event('click');
  Object.defineProperties(event,{target:{value:env.root},clientX:{value:20},clientY:{value:20}});
  env.doc.dispatchEvent(event); await tick();
  env.animations.at(-1).finish(); await tick();
  assert.equal(env.root.dataset.theme,'dark');
  assert.equal(env.attributes.has('aria-busy'),false);
  env.controller.destroy();
});

test('snapshot failure still applies the theme and leaves the button usable',async()=>{
  const env=fixture({supported:true,broken:true}); env.click(); await tick();
  assert.equal(env.root.dataset.theme,'light');
  assert.equal(env.attributes.has('aria-busy'),false);
  env.click(); await tick();
  assert.equal(env.root.dataset.theme,'dark');
  env.controller.destroy();
});

test('effect update failure clears the snapshot and busy state without an unhandled rejection',async()=>{
  const env=fixture({supported:true,effectFails:true}); env.click(); await tick();
  assert.equal(env.root.dataset.theme,'light');
  assert.equal(env.root.dataset.themeTransition,undefined);
  assert.equal(env.attributes.has('aria-busy'),false);
  env.controller.destroy();
});
