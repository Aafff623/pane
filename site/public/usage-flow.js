// Magpie routing motion + Magic UI Animated Beam, adapted to a static site.
// Original sources and MIT notices: site/vendor/{magpie-routing,magicui-animated-beam}.
import { paneMotion } from '/motion.js?v=1';
import { bindMotionToggle } from '/ui.js?v=1';

export function initUsageFlow(isEnglish,en){
  const host=document.querySelector('.why-flow');if(!host)return;
  const stage=host.querySelector('.why-stage'),svg=host.querySelector('.why-wires');
  const work=host.querySelector('.why-work'),hub=host.querySelector('.why-destination');
  const rows=[...host.querySelectorAll('.why-source')],caption=host.querySelector('.why-flow-caption');
  const count=host.querySelector('.why-flow-count b'),pause=host.querySelector('.why-pause');
  const reduce=matchMedia('(prefers-reduced-motion: reduce)'),ns='http://www.w3.org/2000/svg';
  let visible=!('IntersectionObserver' in window),frame=0,previous=0,time=0,next=0,index=0,total=0,trips=[];
  const make=(tag,attrs,parent=svg)=>{const node=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attrs))node.setAttribute(k,v);parent.append(node);return node};
  const defs=make('defs',{});
  const wires=[work,...rows].map((_,i)=>{
    const gradient=make('linearGradient',{id:`why-beam-${i}`,gradientUnits:'userSpaceOnUse',x1:'0%',x2:'0%',y1:'0%',y2:'0%'},defs);
    [['0%','#ffaa40',0],['0%','#ffaa40',1],['32.5%','#9c40ff',1],['100%','#9c40ff',0]].forEach(([offset,color,opacity])=>make('stop',{offset,'stop-color':color,'stop-opacity':opacity},gradient));
    return{path:make('path',{class:'why-wire'}),beam:make('path',{class:'why-beam',stroke:`url(#why-beam-${i})`}),gradient,length:0};
  });
  const text=(zh,english)=>isEnglish?english:zh;
  const insight=host.querySelector('.why-insight');
  if(insight){
    insight.setAttribute('aria-label',text('选中服务的用量摘要','Selected provider usage summary'));
    insight.querySelector('.why-insight-label').textContent=text('当前选中 · 示例数据','SELECTED PROVIDER · SAMPLE DATA');
    insight.querySelector('.why-link span').textContent=text('打开互动工作台','Open interactive workspace');
  }
  // Magic UI's exact cubic-bezier(.16,1,.3,1), solved for x rather than
  // approximating it with a different easing function.
  const easeBeam=x=>{let lo=0,hi=1;for(let i=0;i<14;i++){const t=(lo+hi)/2,v=3*(1-t)**2*t*.16+3*(1-t)*t*t*.3+t**3;if(v<x)lo=t;else hi=t}return 1-(1-(lo+hi)/2)**3};
  function layout(){
    const r=stage.getBoundingClientRect();if(!r.width)return;
    svg.setAttribute('viewBox',`0 0 ${r.width} ${r.height}`);
    const box=node=>{const b=node.getBoundingClientRect();return{l:b.left-r.left,r:b.right-r.left,t:b.top-r.top,b:b.bottom-r.top,x:(b.left+b.right)/2-r.left,y:(b.top+b.bottom)/2-r.top}};
    const h=box(hub),s=box(work);
    wires[0].path.setAttribute('d',h.l>s.r?`M${s.r} ${s.y} L${h.l} ${h.y}`:`M${s.x} ${s.b} L${h.x} ${h.t}`);
    rows.forEach((row,i)=>{const b=box(row),mx=(h.r+b.l)/2;wires[i+1].path.setAttribute('d',b.l>h.r?`M${h.r} ${h.y} C${mx} ${h.y} ${mx} ${b.y} ${b.l} ${b.y}`:`M${h.x} ${h.b} C${h.x} ${h.b+24} ${b.l-14} ${h.b+6} ${b.l-14} ${h.b+28} L${b.l-14} ${b.y-10} Q${b.l-14} ${b.y} ${b.l} ${b.y}`)});
    wires.forEach(w=>{w.length=w.path.getTotalLength();w.beam.setAttribute('d',w.path.getAttribute('d'))});
  }
  function select(i){
    rows.forEach((row,n)=>{row.classList.toggle('is-active',n===i);row.setAttribute('aria-pressed',String(n===i))});
    const name=rows[i].querySelector('b').childNodes[0].textContent.trim();
    caption.textContent=text(`${name} 的示例用量 ${rows[i].dataset.value}，汇到 Pane。`,`${name}: sample usage ${rows[i].dataset.value}, collected in Pane.`);
    if(insight){
      insight.querySelector('.why-insight-provider h3').textContent=name;
      insight.querySelector('.why-insight-provider img').src=rows[i].querySelector('img').getAttribute('src');
      insight.querySelector('.why-insight-value').textContent=rows[i].dataset.value;
      insight.querySelector('.why-insight-metric').textContent=rows[i].querySelector('small').textContent;
      const copy={tokens:['从本地会话日志汇总 Token 用量。','Token usage collected from local session logs.'],quota:['查看已用比例，掌握当前额度。','See how much of the current quota has been used.'],balance:['查看账户余额，留意可用 Credits。','Check the account balance and available credits.']};
      insight.querySelector('.why-insight-copy').textContent=copy[rows[i].dataset.kind][isEnglish?1:0];
    }
  }
  function hit(node){node.classList.remove('is-hit');void node.offsetWidth;node.classList.add('is-hit')}
  function launch(i){select(i);hit(work);const dot=make('circle',{class:'why-packet',r:4});trips.push({i,dot,start:time,phase:-1})}
  function clear(){trips.forEach(tr=>tr.dot.remove());trips=[];wires.forEach(w=>w.path.classList.remove('is-live'))}
  function tick(now){
    frame=0;time+=Math.min(previous?now-previous:0,50);previous=now;
    if(time>=next){launch(index);index=(index+1)%rows.length;next=time+1900}
    const live=new Set();
    trips=trips.filter(tr=>{
      const age=time-tr.start;
      if(age>=1660){tr.dot.remove();count.textContent=String(++total);hit(work);return false}
      const phase=age<380?0:age<800?1:age<980?2:age<1340?3:4;
      if(phase!==tr.phase){if(phase===1||phase===4)hit(hub);if(phase===2)select(tr.i);tr.phase=phase}
      const n=phase===0||phase===4?0:tr.i+1,w=wires[n];live.add(n);
      const start=[0,380,800,980,1340][phase],duration=[380,420,180,360,320][phase];
      const k=phase===2?1:Math.min(1,(age-start)/duration),e=k<.5?2*k*k:1-(-2*k+2)**2/2;
      const back=phase>=3,p=w.path.getPointAtLength((back?1-e:e)*w.length);
      tr.dot.setAttribute('cx',p.x);tr.dot.setAttribute('cy',p.y);tr.dot.classList.toggle('is-return',back);return true;
    });
    wires.forEach((w,n)=>{
      w.path.classList.toggle('is-live',live.has(n));
      // Original 3s beam sweep, orange → purple stops and coordinate range.
      // Branches are staggered so the hub receives a continuous stream.
      const e=easeBeam(((time+n*420)%3000)/3000);
      w.gradient.setAttribute('x1',`${10+100*e}%`);w.gradient.setAttribute('x2',`${100*e}%`);
    });
    frame=requestAnimationFrame(tick);
  }
  function sync(){
    if(frame)cancelAnimationFrame(frame);frame=0;previous=0;
    // Reduced motion stays authoritative even when the visitor un-pauses: the
    // shared switch is a preference, the OS setting is a floor.
    const still=paneMotion.paused||reduce.matches;
    host.classList.toggle('is-paused',still||!visible||document.hidden);
    if(!still&&visible&&!document.hidden)frame=requestAnimationFrame(tick);
  }
  rows.forEach((row,i)=>row.addEventListener('click',()=>{
    clear();select(i);index=i;next=time;
    if(reduce.matches){count.textContent=String(++total);return}
    sync();
  }));
  // The collector's switch is the page-wide switch; the label and aria state
  // come from the shared controller so every pause button on the page agrees.
  bindMotionToggle(pause,paneMotion,paused=>paused?text('恢复动画',en.whyResume):text('暂停动画',en.whyPause));
  paneMotion.subscribe(sync);
  const resize=new ResizeObserver(layout);resize.observe(stage);
  const observer='IntersectionObserver' in window?new IntersectionObserver(entries=>{visible=entries[0].isIntersecting;sync()},{threshold:.1}):null;
  observer?.observe(stage);document.addEventListener('visibilitychange',sync);
  reduce.addEventListener('change',()=>{clear();sync()});
  window.addEventListener('pagehide',()=>{if(frame)cancelAnimationFrame(frame);frame=0});window.addEventListener('pageshow',sync);
  select(0);layout();sync();
}
