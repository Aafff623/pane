import { en } from '/i18n.js';
const frame=document.querySelector('#pane-demo'),settingsFrame=document.querySelector('#pane-settings'),root=document.documentElement;let selected='overview',refreshTimer; let demoTheme='dark';
root.classList.add('motion-ready');
// Kept for the parked bilingual copy. The launched site is Chinese-only, but
// retaining the map here keeps the source ready for the next edge rewrite.
const legacyTranslations = {
  '示例数据 · 交互体验':'Sample data · Interactive preview','这是一组示例数据':'Sample data, real interaction','额度、花费与账户均用于演示。':'Quotas, spend and accounts are sample data.','可以操作，不会连接真实账户。':'Explore freely. No real accounts are connected.','按真实项目分类':'Categories from Pane','边缘轻触，侧栏浮现':'Hover the edge to reveal','服务导航，按需出现':'Provider navigation on demand','桌面端 AI 用量面板':'Your desktop AI usage monitor',
  '功能':'Features','下载':'Download','文档':'Docs','常见问题':'FAQ','立即体验':'Try it','所有 AI 工具的用量':'AI usage for every tool','一个悬浮窗，一眼看清':'One floating window. Everything clear.','连接你的 AI 编程工具 — 查看额度、追踪花费、掌握重置时间，让每一次使用都心中有数。':'Connect your AI coding tools — see quotas, track spend and know when limits reset.','下载 Windows 版':'Download for Windows','支持你的常用工具':'Works with your tools','交互体验':'Interactive preview','重置':'Reset','你的 AI 用量，随时在手边':'Your AI usage, always within reach','点击圆环、切换周期':'Click a ring or change the period','或打开花费详情':'or open spend details','额度总览':'Quota overview','花费详情':'Spend details','设置':'Settings','刷新用量':'Refresh usage','为每天都在用 AI 的你而设计。Pane 把分散的用量信息，变成随时可看的桌面面板。':'Designed for people who use AI every day. Pane turns scattered usage data into a desktop panel you can always see.','额度，一处看清':'Quotas in one view','多个工具、多组账户，汇集到一个总览。会话、周度与月度额度，点击即可切换。':'Bring multiple tools and accounts into one overview. Switch between session, weekly and monthly limits.','花费，有迹可循':'Spend, with a trail','从本地 CLI 日志汇总 Token 与花费。用热力图回看每一天，按工具和模型查看明细。':'Aggregate tokens and spend from local CLI logs. Review every day on a heatmap, by tool and model.','重置，不再猜测':'Resets, without guessing','额度还剩多少、什么时候恢复，一眼可见。接近上限与重置时，桌面提醒帮你留意。':'See what remains and when it resets. Desktop alerts call out limits and upcoming resets.','密钥，留在本机':'Keys stay local','密钥存储在你的电脑上，查询直连对应厂商。开源代码可审阅，匿名遥测可关闭。':'Keys stay on your computer and queries go directly to each provider. The code is open to review and anonymous telemetry is optional.','你可能想知道':'Questions, answered','你的 AI 用量，现在尽在眼前。':'Your AI usage, right in front of you.','下载 Pane':'Download Pane','先体验一下':'Try the demo','源代码':'Source','反馈':'Feedback'
};

const isEnglish = location.pathname === '/en' || location.pathname.startsWith('/en/') || new URLSearchParams(location.search).get('lang') === 'en';
const demoLang = isEnglish ? 'en' : 'zh';
function setText(selector, key, html = false) { const node = document.querySelector(selector); if (node && isEnglish && en[key]) html ? node.innerHTML = en[key] : node.textContent = en[key]; }
function setTexts(selector, keys, html = false) { document.querySelectorAll(selector).forEach((node, i) => { const value = en[keys[i]]; if (node && isEnglish && value) html ? node.innerHTML = value : node.textContent = value; }); }
function applyEnglishCopy() {
  if (!isEnglish) return;
  document.documentElement.lang = 'en';
  document.title = "Pane — AI usage workspace";
  setText('.nav-context', 'navContext'); setTexts('.nav-links a', ['navDemo','navFeatures','navFaq']); setText('#language', 'language'); setText('.nav-actions .compact', 'navDownload');
  setTexts('.hero-meta span', ['heroMetaLeft','heroMetaRight']); setText('.eyebrow', 'heroEyebrow');
  document.querySelector('.title-art').innerHTML = '<span class="title-lead">Every AI tool\'s quota</span><em class="title-focus"><span class="title-outline">and spend,</span><span class="title-solid">at a glance.</span></em>';
  setText('.hero-description', 'heroDesc'); setText('.hero-download', 'heroDownload'); setText('.text-link', 'heroDemo'); setTexts('.hero-stats span', ['providers','credentials','shortcut']); setText('.hero-stats>div:nth-child(2) b','localCredentials');
  setTexts('.agent-portrait span, .note-kicker', ['live','portraitLabel']); document.querySelectorAll('.agent-slide').forEach((img,i)=>img.alt=['Pane companion seated with a usage dashboard','Pane companion standing with a quota ring','Pane robot companion surrounded by usage charts'][i]); document.querySelectorAll('.agent-dots button').forEach((b,i)=>b.setAttribute('aria-label',`Show illustration ${i+1}`)); setText('.blueprint-note p', 'portraitCopy', true); setText('.note-foot', 'portraitNote'); setText('.stage-rail>span','workflow'); setText('.keyboard-demo>span','keyboardKicker'); setText('.config-kicker','runtimeKicker');
  setText('.download-section .section-index','downloadIndex'); setText('.demo-section .section-head h2', 'demoTitle'); setText('.demo-section .section-head p', 'demoDesc', true); setText('.demo-status', 'demoStatus'); setText('#reset-demo', 'reset'); setText('.workflow-legend>.legend-kicker', 'how'); setText('.legend-heading b', 'workflow'); setText('.legend-heading small', 'shortcuts');
  setTexts('.shortcut-list span', ['alt2','shift','ctrls','esc']); setTexts('.legend-capabilities .legend-heading b, .legend-capabilities .legend-heading small, .legend-desc', ['capabilities','capabilitiesKicker','capabilitiesCopy']); setText('.legend-ecosystem b','ecosystem'); setText('.legend-ecosystem span','ecosystemCopy'); setText('.legend-foot','sample');
  setTexts('.keyboard-demo button b', ['toggle','close','period','category','settingsShortcut','customize','themeShortcut','refreshShortcut','expiring']); setTexts('.stage-config dt', ['runtime','runtimeSettings']); setTexts('.stage-config dd', ['runtimeCopy','runtimeSettingsCopy']);
  const notes=[['.annotation-sample strong','annSample'],['.annotation-sample>span:not(.annotation-number)','annSampleCopy'],['.annotation-left strong','annEdge'],['.annotation-left>span','annEdgeCopy'],['.annotation-right strong','annProviders'],['.annotation-right>span','annProvidersCopy'],['.annotation-tabs strong','annTabs'],['.annotation-tabs>span:not(.annotation-number)','annTabsCopy'],['.annotation-periods strong','annPeriods'],['.annotation-periods>span:not(.annotation-number)','annPeriodsCopy'],['.annotation-accounts strong','annAccounts'],['.annotation-accounts>span:not(.annotation-number)','annAccountsCopy'],['.annotation-spend strong','annSpend'],['.annotation-spend>span:not(.annotation-number)','annSpendCopy'],['.annotation-pool strong','annPool'],['.annotation-pool>span','annPoolCopy'],['.annotation-groups strong','annGroups'],['.annotation-groups>span:not(.annotation-number)','annGroupsCopy'],['.annotation-status strong','annStatus'],['.annotation-status>span','annStatusCopy'],['.annotation-settings strong','annSettings'],['.annotation-settings>span:not(.annotation-number)','annSettingsCopy'],['.annotation-refresh strong','annRefresh'],['.annotation-refresh>span','annRefreshCopy']]; notes.forEach(([s,k])=>setText(s,k,k.endsWith('Copy')));
  setTexts('.demo-controls button', ['overview','spend','settings','refresh']); setText('.demo-rest>b','restTitle'); setText('.demo-rest p','restCopy'); setText('.keyboard-demo>span','keyboardKicker'); setText('.keyboard-demo h3','keyboardTitle'); setText('.keyboard-demo>p','keyboardCopy'); setText('#keyboard-start','start'); setText('.keyboard-feedback','keyboardNote'); setText('.keyboard-demo>small','keyboardHint');
  setText('.provider-section .section-head h2','providersTitle'); setText('.provider-section .section-head p','providersCopy',true); setText('.feature-section .section-head h2','featuresTitle'); setText('.feature-section .section-head p','featuresCopy'); setTexts('.feature-grid h3',['quotaTitle','spendTitle','resetTitle','privacyTitle']); setTexts('.feature-grid p',['quotaCopy','spendCopy','resetCopy','privacyCopy']); setText('.faq-section .section-head p','faqCopy'); setTexts('.faq-section summary',['faq.q1','faq.q2','faq.q3','faq.q4','faq.q5','faq.q6']); setTexts('.faq-section details p',['faq.a1','faq.a2','faq.a3','faq.a4','faq.a5','faq.a6']); setText('.download-section h2','downloadTitle'); setTexts('.platform-disabled small',['coming','coming']); setText('.bottom-actions .secondary','tryDemo'); setText('.platforms','downloadNote'); setTexts('.site-footer .footer-content>div a',['source','docs','feedback']);
}
applyEnglishCopy();
// The hero uses a small, art-directed carousel instead of a looping GIF.
const agentSlides=[...document.querySelectorAll('.agent-slide')];
const agentDots=[...document.querySelectorAll('.agent-dots button')];
let agentIndex=0, agentTimer;
function showAgentSlide(index){if(!agentSlides.length)return;agentIndex=(index+agentSlides.length)%agentSlides.length;agentSlides.forEach((slide,i)=>slide.classList.toggle('is-active',i===agentIndex));agentDots.forEach((dot,i)=>{dot.classList.toggle('is-active',i===agentIndex);dot.setAttribute('aria-current',i===agentIndex?'true':'false')})}
function restartAgentCarousel(){clearInterval(agentTimer);if(agentSlides.length>1&&!matchMedia('(prefers-reduced-motion: reduce)').matches)agentTimer=setInterval(()=>showAgentSlide(agentIndex+1),5200)}
agentDots.forEach((dot,i)=>dot.addEventListener('click',()=>{showAgentSlide(i);restartAgentCarousel()}));showAgentSlide(0);restartAgentCarousel();
document.querySelector('#language')?.addEventListener('click',()=>{location.href=isEnglish?'/':'/?lang=en'});

// Keep margin annotations attached to real demo geometry as cards and tabs move.
let demoAnchors;
function positionAnnotations() {
  if (!demoAnchors || innerWidth <= 950 || frame.parentElement.hidden) return;
  const stage=document.querySelector('.demo-desktop'), bounds=stage.getBoundingClientRect(), app=frame.getBoundingClientRect();
  const scaleX=app.width/(demoAnchors.width||app.width), scaleY=app.height/(demoAnchors.height||app.height);
  // Resolve the controls in the same-origin demo frame. The generic fallback
  // used to map settings to the side zone, so the settings note could point
  // at a provider card instead of the actual gear button.
  try {
    const demoDocument=frame.contentDocument;
    const knownControls={settings:'#settings-btn',refresh:'#refresh',status:'#status'};
    for(const [name,selector] of Object.entries(knownControls)){
      const control=demoDocument?.querySelector(selector), rect=control?.getBoundingClientRect();
      if(control&&rect&&rect.width&&rect.height)demoAnchors.anchors[name]={x:rect.left,y:rect.top,width:rect.width,height:rect.height};
    }
  }catch{}
  stage.classList.toggle('is-settings',demoAnchors.settings);
  for (const [name, anchor] of Object.entries(demoAnchors.anchors)) {
    const note=document.querySelector(`[data-anchor="${name}"]`);
    if (!note) continue;
    if (!anchor) { note.classList.add('is-outside'); note.querySelector('.annotation-connector')?.style.setProperty('width','0px'); continue; }
    const targetX=app.left-bounds.left + (anchor.x+anchor.width/2)*scaleX;
    const targetY=app.top-bounds.top + (anchor.y+anchor.height/2)*scaleY;
    const visible=anchor.y+anchor.height/2>=0 && anchor.y+anchor.height/2<=demoAnchors.height;
    note.classList.toggle('is-outside',!visible);
  }
  avoidAnnotationCollisions();
  // Collision avoidance changes card positions. Draw connectors only after
  // that final layout pass, otherwise arrows keep pointing at stale boxes.
  for (const [name, anchor] of Object.entries(demoAnchors.anchors)) {
    const note=document.querySelector(`[data-anchor="${name}"]`);
    if (!note || !anchor || note.classList.contains('is-outside')) continue;
    const targetX=app.left-bounds.left + (anchor.x+anchor.width/2)*scaleX;
    const targetY=app.top-bounds.top + (anchor.y+anchor.height/2)*scaleY;
    const noteBox=note.getBoundingClientRect(), line=note.querySelector('.annotation-connector');
    if (!line) continue;
    const isLeft=name==='sample' || name==='left' || name==='spend' || name==='pool' || name==='accounts' || name==='refresh';
    const startX=(isLeft?noteBox.right:noteBox.left)-bounds.left;
    const startY=noteBox.top-bounds.top+noteBox.height/2;
    const dx=targetX-startX,dy=targetY-startY;
    line.style.width=`${Math.hypot(dx,dy)}px`;
    line.style.left=isLeft?'100%':'0'; line.style.right='auto';
    line.style.top=`${noteBox.height/2}px`; line.style.transformOrigin='left center';
    line.style.transform=`rotate(${Math.atan2(dy,dx)*180/Math.PI}deg)`;
  }
}
function avoidAnnotationCollisions(){
  const stage=document.querySelector('.demo-desktop');
  for(const side of [['left'],['right']]){
    let previousBottom=-Infinity;
    const notes=noteNodes.filter(n=>n.style.display!== 'none'&&!n.classList.contains('is-outside')&&n.matches(side[0]==='left'?'.annotation-sample,.annotation-left,.annotation-spend,.annotation-pool,.annotation-accounts,.annotation-refresh':'.annotation-right,.annotation-tabs,.annotation-groups,.annotation-status,.annotation-periods,.annotation-settings')).sort((a,b)=>a.getBoundingClientRect().top-b.getBoundingClientRect().top);
    for(const note of notes){
      note.style.translate='none';
      const box=note.getBoundingClientRect(), shift=Math.max(0,previousBottom-box.top+14);
      if(shift)note.style.translate=`0 ${shift}px`;
      previousBottom=box.bottom+shift;
    }
  }
}
window.addEventListener('message',event=>{
  if(event.source!==frame.contentWindow || event.origin!==location.origin)return;
  if(event.data?.type==='pane-demo-anchors'){demoAnchors=event.data;positionAnnotations();return}
  // The panel's own gear button asks the host to reveal the settings window.
  if(event.data?.type==='pane-demo'&&event.data.action==='settings')setSettingsOverlay(true);
});
// (stage size changes are covered by the rAF follower below)
let demoFrame=0;function tickAnnotations(){positionAnnotations();updateNoteAct();demoFrame=requestAnimationFrame(tickAnnotations)}
// Entry animations move the stage without resizing it; reproject the latest
// anchor message every frame while the demo section is near the viewport.
const demoTicker=new IntersectionObserver(entries=>{for(const entry of entries){if(entry.isIntersecting){if(!demoFrame)demoFrame=requestAnimationFrame(tickAnnotations)}else if(demoFrame){cancelAnimationFrame(demoFrame);demoFrame=0}}},{rootMargin:'120px'});
demoTicker.observe(document.querySelector('.demo-desktop'));
// Two annotation acts: the overview set when the stage enters, the detail
// set (spend / pool / groups / status) as the visitor scrolls deeper.
// display is driven by JS directly — a CSS visibility transition whose
// target state is active from first paint never resolves in Chromium.
const desktop=document.querySelector('.demo-desktop');
const noteNodes=[...document.querySelectorAll('.demo-annotation')];
 desktop.dataset.mode='auto';
const modeAnchors={overview:new Set(['sample','left','right','tabs','periods','accounts']),spend:new Set(['spend','pool','groups','status']),settings:new Set(['settings','status']),refresh:new Set(['refresh','status'])};
function updateNoteAct(){
  const r=desktop.getBoundingClientRect();
  const depth=(innerHeight-r.top)/(innerHeight+r.height);
  const showB=depth>=0.5;
  const mode=desktop.dataset.mode;
  const activeAnchors=modeAnchors[mode]||modeAnchors[showB?'spend':'overview'];
  desktop.dataset.notes=showB?'b':'a';
  // Keep the annotations tied to the active panel view. Previously the
  // overview/spend groups were only driven by scroll depth, so clicking
  // Settings or Refresh left unrelated cards visible and made the guidance
  // feel detached from the control the visitor had just selected.
  for(const n of noteNodes){
    const anchor=n.dataset.anchor;
    n.style.display=activeAnchors.has(anchor)?'':'none';
  }
}
addEventListener('scroll',updateNoteAct,{passive:true});
addEventListener('resize',updateNoteAct,{passive:true});
updateNoteAct();
// Title art: split into per-character spans so the entrance can stagger.
document.querySelectorAll('.title-art').forEach(t=>{
  const walk=n=>{[...n.childNodes].forEach(c=>{
    if(c.nodeType===3&&c.textContent.trim()){const frag=document.createDocumentFragment();let ci=0;for(const ch of c.textContent){const s=document.createElement('span');s.className=ch===' '?'char char-space':'char';s.style.setProperty('--i',ci);s.style.setProperty('--dy',((ci%3)-1)*2+'px');s.style.setProperty('--rot',((ci%4)-1.5)*.7+'deg');ci++;s.textContent=ch===' '?'\u00a0':ch;frag.append(s)}c.replaceWith(frag)}
    else if(c.nodeType===1)walk(c);
  })};
  walk(t);
});
const settingsButton=document.querySelector('[data-demo=settings]');const settingsWrap=document.querySelector('#pane-settings-wrap');function setSettingsOverlay(open){if(open&&settingsWrap.hidden){if(!settingsFrame.dataset.loaded){settingsFrame.src=`/demo/index.html?lang=${demoLang}&theme=${demoTheme}&settings=1`;settingsFrame.dataset.loaded='1'}settingsWrap.hidden=false}if(!open)settingsWrap.hidden=true;settingsButton.classList.toggle('active',!settingsWrap.hidden)}settingsButton.addEventListener('click',()=>setSettingsOverlay(settingsWrap.hidden));document.querySelector('#pane-settings-close').addEventListener('click',()=>setSettingsOverlay(false));function send(action){frame.contentWindow?.postMessage({type:'pane-demo',action},location.origin)}function loadDemo(){frame.src=`/demo/index.html?lang=${demoLang}&theme=${demoTheme}`;settingsWrap.hidden=true;settingsFrame.removeAttribute('src');delete settingsFrame.dataset.loaded;selected='overview';desktop.dataset.mode='auto';document.querySelectorAll('[data-demo]').forEach(b=>b.classList.toggle('active',b.dataset.demo===selected))}loadDemo();document.querySelector('#theme').addEventListener('click',()=>{root.dataset.theme=root.dataset.theme==='light'?'dark':'light';try{localStorage.setItem('pane-site-theme',root.dataset.theme)}catch{};document.querySelector('#theme img').src=`/icons/${root.dataset.theme==='light'?'moon':'sun'}.svg`});document.querySelector('#reset-demo').addEventListener('click',loadDemo);document.querySelectorAll('[data-demo]').forEach(b=>b.addEventListener('click',()=>{const a=b.dataset.demo;if(a==='settings'){desktop.dataset.mode='settings';updateNoteAct();return}if(a!=='refresh')setSettingsOverlay(false);if(a==='refresh'){desktop.dataset.mode='refresh';updateNoteAct();b.disabled=true;send(a);clearTimeout(refreshTimer);refreshTimer=setTimeout(()=>b.disabled=false,650);return}selected=a;desktop.dataset.mode=a;send(a);document.querySelectorAll('[data-demo]').forEach(x=>x.classList.toggle('active',x.dataset.demo===selected));updateNoteAct()}));document.querySelectorAll('details').forEach(d=>d.addEventListener('toggle',()=>{if(d.open)document.querySelectorAll('details').forEach(x=>{if(x!==d)x.open=false})}));

const gsap = window.gsap;
if (gsap && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
  const ctx = gsap.context(() => {
    // Hero entrance lives in CSS (site.css keyframes): gsap.from() hides its
    // targets until the first rAF tick, and a throttled/backgrounded window
    // then shows a blank first screen. CSS animation always runs.
    const reveal = new IntersectionObserver(entries => entries.forEach(entry => {
      if (!entry.isIntersecting) return;
      gsap.fromTo(entry.target, { y: 22, opacity: 0 }, { y: 0, opacity: 1, duration: .65, ease: 'power3.out', delay: Number(entry.target.dataset.motionDelay || 0) });
      reveal.unobserve(entry.target);
    }), { threshold: .14, rootMargin: '0px 0px -8%' });
    document.querySelectorAll('.feature-grid article, .faq-section details, .download-section .button, .provider-section .section-head').forEach((el, i) => { el.dataset.motionDelay = String(Math.min(i * .045, .22)); reveal.observe(el); });
    // Duplicate each inventory once and move the two complete belts forever.
    // The second row starts offset and travels in the opposite direction, so
    // the handoff is continuous instead of jumping one card at a time.
    document.querySelectorAll('.marquee-track').forEach((track,index)=>{
      if (!track.dataset.looped) {
        [...track.children].forEach(item => track.append(item.cloneNode(true)));
        track.dataset.looped='true';
      }
      track.classList.add(index ? 'belt-reverse' : 'belt-forward');
    });
    // Continuous idle loops for the four feature legends (user request: the
    // fourth module should keep animating, not just play its entrance once).
    document.querySelectorAll('.feature-art').forEach(art => {
      const rings = art.querySelectorAll('.mini-ring');
      const bars = art.querySelectorAll('.bar-art i');
      const clock = art.querySelector('.clock-face');
      const shield = art.querySelector('.shield');
      if (rings.length === 3) {
        gsap.to(rings[0], { y: '+=7', rotation: 7, duration: 2.2, yoyo: true, repeat: -1, ease: 'sine.inOut' });
        gsap.to(rings[1], { y: '-=7', rotation: -6, duration: 2.6, yoyo: true, repeat: -1, ease: 'sine.inOut' });
        gsap.to(rings[2], { y: '+=7', rotation: 5, duration: 2.4, yoyo: true, repeat: -1, ease: 'sine.inOut' });
      }
      if (bars.length) bars.forEach((bar, i) => gsap.fromTo(bar, { scaleY: .72 }, { scaleY: 1, duration: .9 + i * .13, yoyo: true, repeat: -1, ease: 'sine.inOut', transformOrigin: 'bottom' }));
      if (clock) gsap.to(clock, { rotation: 9, y: -3, duration: 1.6, yoyo: true, repeat: -1, ease: 'sine.inOut' });
      if (shield) gsap.to(shield, { y: '-=9', duration: 1.8, yoyo: true, repeat: -1, ease: 'sine.inOut' });
      art.querySelectorAll('.quota-bars em').forEach((fill,i)=>gsap.fromTo(fill,{scaleX:.18},{scaleX:1,duration:1.1+i*.18,delay:i*.12,yoyo:true,repeat:-1,ease:'sine.inOut',transformOrigin:'left center'}));
    });
    document.querySelectorAll('[data-tilt-card]').forEach(card => {
      card.addEventListener('pointermove', event => { const r = card.getBoundingClientRect(); gsap.to(card, { rotateY: ((event.clientX-r.left)/r.width-.5)*4, rotateX: -((event.clientY-r.top)/r.height-.5)*4, duration: .25, overwrite: true }); });
      card.addEventListener('pointerleave', () => gsap.to(card, { rotateX: 0, rotateY: 0, duration: .45, ease: 'power2.out' }));
    });
  });
  window.addEventListener('pagehide', () => ctx.revert(), { once: true });
}

document.querySelectorAll('[data-tilt-card]').forEach(card=>{const reset=()=>{card.style.setProperty('--tilt-x','0deg');card.style.setProperty('--tilt-y','0deg');card.style.setProperty('--spot-x','50%');card.style.setProperty('--spot-y','30%')};card.addEventListener('pointermove',event=>{const r=card.getBoundingClientRect(),x=(event.clientX-r.left)/r.width-.5,y=(event.clientY-r.top)/r.height-.5;card.style.setProperty('--tilt-x',`${x*8}deg`);card.style.setProperty('--tilt-y',`${y*-8}deg`);card.style.setProperty('--spot-x',`${(x+.5)*100}%`);card.style.setProperty('--spot-y',`${(y+.5)*100}%`)});card.addEventListener('pointerleave',reset);reset()});
const revealObserver='IntersectionObserver' in window?new IntersectionObserver(entries=>entries.forEach(entry=>{if(entry.isIntersecting){entry.target.classList.add('is-visible');revealObserver?.unobserve(entry.target)}}),{threshold:.14,rootMargin:'0px 0px -8%'}):null;
document.querySelectorAll('.demo-stage,.integrations,.feature-section,.faq-section,.download-section,.site-footer').forEach((element,index)=>{element.style.setProperty('--reveal-delay',`${Math.min(index*45,180)}ms`);revealObserver?.observe(element);if(!revealObserver)element.classList.add('is-visible')});

// Initialize after all host bindings. No earlier loadDemo() call depends on
// this controller, and every callback sees initialized state.
function initKeyboardDemo() {
  const region = document.querySelector('.keyboard-demo');
  const stage = document.querySelector('.blueprint-stage');
  const start = document.querySelector('#keyboard-start');
  const feedback = document.querySelector('.keyboard-feedback');
  const app = frame.parentElement;
  const rest = document.querySelector('.demo-rest');
  const periods = { '5h': '5 小时', week: '7 天', month: '每月' };
  // Every shortcut card is a button with the same name as its action, so the
  // key table, the click bindings and the disabled state stay in one place.
  const buttons = {
    toggle: document.querySelector('#keyboard-toggle'),
    close: document.querySelector('#keyboard-close'),
    period: document.querySelector('#keyboard-period'),
    category: document.querySelector('#keyboard-category'),
    settings: document.querySelector('#keyboard-settings'),
    customize: document.querySelector('#keyboard-customize'),
    theme: document.querySelector('#keyboard-theme'),
    refresh: document.querySelector('#keyboard-refresh'),
    expiring: document.querySelector('#keyboard-expiring'),
  };
  let active = false, visible = true, shiftAlone = false, period = '5h';
  // A shortcut's own line ("已切到…") outlives the burst of state re-renders
  // the demo emits while it repaints, then falls back to the state line.
  let note = '', noteTimer;
  const transport = (action, extra = {}) => frame.contentWindow?.postMessage({ type: 'pane-demo', action, ...extra }, location.origin);
  function render() {
    app.hidden = !visible;
    rest.hidden = visible;
    desktop.classList.toggle('is-panel-hidden', !visible);
    region.classList.toggle('is-active', active);
    start.textContent = active ? (isEnglish ? 'End interaction ↗' : '结束交互 ↗') : (isEnglish ? 'Start interaction ↗' : '点击开始交互 ↗');
    start.setAttribute('aria-pressed', String(active));
    for (const [name, button] of Object.entries(buttons)) button.disabled = !active || (!visible && name !== 'toggle');
    feedback.textContent = note || (active ? (visible ? (isEnglish ? `Panel visible · ${periods[period]}` : `面板已浮现 · 当前 ${periods[period]}`) : (isEnglish ? 'Panel dismissed · press Alt + 2 or use a key above' : '面板已收起 · 按 Alt + 2 或上方按键唤出')) : (isEnglish ? 'Start here, then try a shortcut.' : '点击开始，再试试快捷键。'));
    positionAnnotations();
  }
  function say(text) {
    if (!active) return;
    note = text;
    feedback.textContent = text;
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => { note = ''; render(); }, 2800);
  }
  const actions = {
    toggle() {
      if (!active) return;
      visible = !visible;
      if (!visible) { setSettingsOverlay(false); region.focus({ preventScroll: true }); }
      render();
      if (!visible) say('面板已收起 · Alt + 2 再唤出');
    },
    close() { if (visible) actions.toggle(); },
    period() { if (visible) transport('cycle-period'); },
    category() { if (visible) { transport('cycle-category'); say('正在切换分类…'); } },
    settings() { if (visible) { setSettingsOverlay(true); say('设置面板已打开 · Ctrl + S'); } },
    customize() { if (visible) { transport('customize'); say('自定义抽屉已展开 · Ctrl + E'); } },
    theme() { if (visible) { transport("theme"); say(isEnglish ? "Panel theme toggled · Ctrl + L" : "面板主题已切换 · Ctrl + L"); } },
    refresh() { if (visible) { transport('refresh'); say('已刷新用量 · Ctrl + R'); } },
    expiring() { if (visible) { transport('expiring'); say('临期清单已切换 · T'); } },
  };
  const keys = [
    ['toggle', e => e.code === 'Digit2' && e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey],
    ['category', e => e.code === 'Digit1' && e.shiftKey && !e.altKey && !e.ctrlKey && !e.metaKey],
    ['settings', e => e.code === 'KeyS' && e.ctrlKey && !e.altKey && !e.shiftKey],
    ['customize', e => e.code === 'KeyE' && e.ctrlKey && !e.altKey && !e.shiftKey],
    ['theme', e => e.code === 'KeyL' && e.ctrlKey && !e.altKey && !e.shiftKey],
    ['refresh', e => e.code === 'KeyR' && e.ctrlKey && !e.altKey && !e.shiftKey],
    ['expiring', e => e.code === 'KeyT' && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey],
  ];
  function stop() { active = false; visible = true; shiftAlone = false; transport('keyboard-demo', { enabled: false }); render(); }
  start.addEventListener('click', () => {
    if (active) { stop(); return; }
    active = true; visible = false; shiftAlone = false;
    setSettingsOverlay(false);
    transport('overview');
    transport('keyboard-demo', { enabled: true });
    render(); region.focus({ preventScroll: true });
  });
  for (const [name, button] of Object.entries(buttons)) button.addEventListener('click', () => actions[name]());
  function inScope(event) {
    return active && stage.contains(document.activeElement) && settingsWrap.hidden && !event.target.closest('input,textarea,select,[contenteditable="true"]');
  }
  window.addEventListener('keydown', event => {
    if (!inScope(event)) { shiftAlone = false; return; }
    if (event.repeat) return;
    if (event.key === 'Shift') { shiftAlone = !event.ctrlKey && !event.altKey && !event.metaKey; return; }
    shiftAlone = false;
    if (event.code === 'Escape') { event.preventDefault(); actions.close(); return; }
    for (const [name, match] of keys) if (match(event)) { event.preventDefault(); actions[name](); return; }
  });
  window.addEventListener('keyup', event => {
    if (event.key !== 'Shift') return;
    const shouldCycle = shiftAlone && inScope(event) && visible;
    shiftAlone = false;
    if (shouldCycle) { event.preventDefault(); transport('cycle-period'); }
  });
  window.addEventListener('blur', () => { shiftAlone = false; });
  region.addEventListener('focusout', () => { shiftAlone = false; });
  window.addEventListener('message', event => {
    if (event.source !== frame.contentWindow || event.origin !== location.origin) return;
    if (event.data?.type === 'pane-demo-period' && Object.hasOwn(periods, event.data.period)) { period = event.data.period; render(); }
    if (event.data?.type === 'pane-demo-category' && event.data.label) say(`已切到「${event.data.label}」分类 · Shift + 1`);
    if (event.data?.type === 'pane-demo-toggle') actions.toggle();
    if (event.data?.type === 'pane-demo-hide' && active && visible) actions.toggle();
    if (event.data?.type === 'pane-demo-ready') transport('keyboard-demo', { enabled: active });
  });
  document.querySelector('#reset-demo').addEventListener('click', stop);
  render();
}
initKeyboardDemo();
