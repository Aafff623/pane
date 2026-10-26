import { en } from '/i18n.js?v=2';
import { initSiteEffects } from '/effects.js?v=11';
import { initThemeSwap } from '/theme-swap.js?v=4';
import { initUsageFlow } from '/usage-flow.js?v=5';
import { paneMotion } from '/motion.js?v=1';
import { initHeroReactions } from '/hero-reactions.js?v=3';
import { initLineSidebar } from '/line-sidebar.js?v=2';
import { initHeaderNav } from '/header-nav.js?v=2';
import { formatByKind, warmHover, isHoverWarm, onLongPress, moveRovingFocus, bindMotionToggle } from '/ui.js?v=1';
const frame=document.querySelector('#pane-demo'),settingsFrame=document.querySelector('#pane-settings'),root=document.documentElement;let selected='overview',refreshTimer; let demoTheme='dark';
root.classList.add('motion-ready');
// Older copy retained for the text-node translations below.
const legacyTranslations = {
  '示例数据 · 交互体验':'Sample data · Interactive preview','这是一组示例数据':'Sample data, real interaction','额度、花费与账户均用于演示。':'Quotas, spend and accounts are sample data.','可以操作，不会连接真实账户。':'Explore freely. No real accounts are connected.','按真实项目分类':'Categories from Pane','边缘轻触，侧栏浮现':'Hover the edge to reveal','服务导航，按需出现':'Provider navigation on demand','桌面端 AI 用量面板':'Your desktop AI usage monitor',
  '功能':'Features','下载':'Download','文档':'Docs','常见问题':'FAQ','立即体验':'Try it','所有 AI 工具的用量':'AI usage for every tool','一个悬浮窗，一眼看清':'One floating window. Everything clear.','连接你的 AI 编程工具 — 查看额度、追踪花费、掌握重置时间，让每一次使用都心中有数。':'Connect your AI coding tools — see quotas, track spend and know when limits reset.','下载 Windows 版':'Download for Windows','支持你的常用工具':'Works with your tools','交互体验':'Interactive preview','重置':'Reset','你的 AI 用量，随时在手边':'Your AI usage, always within reach','点击圆环、切换周期':'Click a ring or change the period','或打开花费详情':'or open spend details','额度总览':'Quota overview','花费详情':'Spend details','设置':'Settings','刷新用量':'Refresh usage','为每天都在用 AI 的你而设计。Pane 把分散的用量信息，变成随时可看的桌面面板。':'Designed for people who use AI every day. Pane turns scattered usage data into a desktop panel you can always see.','额度，一处看清':'Quotas in one view','多个工具、多组账户，汇集到一个总览。会话、周度与月度额度，点击即可切换。':'Bring multiple tools and accounts into one overview. Switch between session, weekly and monthly limits.','花费，有迹可循':'Spend, with a trail','从本地 CLI 日志汇总 Token 与花费。用热力图回看每一天，按工具和模型查看明细。':'Aggregate tokens and spend from local CLI logs. Review every day on a heatmap, by tool and model.','重置，不再猜测':'Resets, without guessing','额度还剩多少、什么时候恢复，一眼可见。接近上限与重置时，桌面提醒帮你留意。':'See what remains and when it resets. Desktop alerts call out limits and upcoming resets.','密钥，留在本机':'Keys stay local','密钥存储在你的电脑上，查询直连对应厂商。开源代码可审阅，应用不含遥测。':'Keys stay on your computer and queries go directly to each provider. The code is open to review and the app sends no telemetry.','你可能想知道':'Questions, answered','你的 AI 用量，现在尽在眼前。':'Your AI usage, right in front of you.','下载 Pane':'Download Pane','先体验一下':'Try the demo','源代码':'Source','反馈':'Feedback'
};

const isEnglish = location.pathname === '/en' || location.pathname.startsWith('/en/') || new URLSearchParams(location.search).get('lang') === 'en';
// Every pause control on the page names the same shared state the same way.
const motionLabel = paused => paused ? (isEnglish ? en.whyResume : '恢复动画') : (isEnglish ? en.whyPause : '暂停动画');
const demoLang = isEnglish ? 'en' : 'zh';
function setText(selector, key, html = false) { const node = document.querySelector(selector); if (!node || !isEnglish) return; if (!(key in en)) { console.warn('[pane-site] missing i18n key', key); return; } html ? node.innerHTML = en[key] : node.textContent = en[key]; }
function setTexts(selector, keys, html = false) { document.querySelectorAll(selector).forEach((node, i) => { const key = keys[i]; if (!node || !isEnglish || !key) return; if (!(key in en)) { console.warn('[pane-site] missing i18n key', key); return; } html ? node.innerHTML = en[key] : node.textContent = en[key]; }); }
function applyEnglishCopy() {
  if (!isEnglish) return;
  document.documentElement.lang = 'en';
  document.title = en.metaTitle;
  for (const selector of ['meta[name="description"]', 'meta[property="og:description"]', 'meta[name="twitter:description"]']) document.querySelector(selector)?.setAttribute('content', en.metaDescription);
  for (const selector of ['meta[property="og:title"]', 'meta[name="twitter:title"]']) document.querySelector(selector)?.setAttribute('content', en.metaTitle);
  document.querySelector('link[rel="canonical"]')?.setAttribute('href', 'https://pane.threetwoa.live/en/');
  document.querySelector('meta[property="og:url"]')?.setAttribute('content', 'https://pane.threetwoa.live/en/');
  document.querySelector('meta[property="og:locale"]')?.setAttribute('content', 'en_US');
  const heroLead = document.querySelector('.hero-title-lead');
  if (heroLead?.firstChild?.nodeType === Node.TEXT_NODE) heroLead.firstChild.textContent = en.heroTitleLead;
  setText('.hero-title-lead .ink-mark', 'heroTitleCall');
  setText('.hero-title-focus .ink-mark', 'heroTitleVisible');
  setText('.nav-context .chip__label', 'navContext'); setTexts('.nav-links a', ['navDemo','navFeatures','navFaq']); setText('#language', 'language'); setText('.nav-actions .button', 'navDownload');
  // Icon-only controls need a name in whichever language the page is showing.
  for(const [selector,label] of [['.nav-actions .github','GitHub repository'],['.nav-links','Page sections'],['#language','Switch language'],['#theme','Switch theme'],['#pane-settings-close','Close settings window']]){
    const node=document.querySelector(selector);
    if(node)node.setAttribute('aria-label',label);
  }
  setTexts('.hero-meta span', ['heroMetaLeft','heroMetaRight']); setText('.eyebrow .chip__label', 'heroEyebrow');
  setText('.hero-description', 'heroDesc'); setTexts('[data-download-label]', ['heroDownload','heroDownload']); setText('.text-link', 'heroDemo'); setTexts('.hero-stats span', ['providers','credentials','shortcut']); setText('.hero-stats>div:nth-child(2) b','localCredentials');
  setText('.why-kicker','whyKicker'); setText('#why-pane-title','whyTitle'); setText('.why-copy p','whyCopy'); setText('.why-link','whyLink'); setText('.why-destination small','whyDestination');
  setText('.why-work b','whyWork'); setText('.why-work small','whyFocus'); setText('.why-flow-label','whyFlowLabel'); setText('.why-pause','whyPause'); setText('.why-flow-caption','whyFlowCaption'); setText('.why-flow-note','whyFlowNote'); setText('.why-flow-count small','whyFlowCount'); setTexts('.why-source small',['whyClaude','whyCodex','whyCursor','whyRouter','whyTavily']);
  setText('.ecosystem-intro', 'ecosystemIntro');
  setText('#boundaries .section-index','boundariesIndex'); setText('#boundaries-title','boundariesTitle'); setText('#boundaries .section-head p','boundariesCopy');
  setTexts('#boundaries .boundary-card h3',['boundary1Title','boundary2Title','boundary3Title']);
  setTexts('#boundaries .boundary-card p',['boundary1Copy','boundary2Copy','boundary3Copy'],true);
  setTexts('#boundaries .boundary-stat span',['boundaryStatProviders','boundaryStatAccounts','boundaryStatLicense']);
  setText('#boundaries .boundary-stat .why-link','boundaryStatLink',true);
  const ecosystemCopy={'.ecosystem-copy .why-kicker':'PANE ECOSYSTEM / AVAILABLE TODAY','#ecosystem-title':'Usage from your everyday tools. One place to check.','.ecosystem-art-foot>span':'Different services. One entry point.','.ecosystem-core>span':'Connect scattered usage','.ecosystem-outlook':'4 ecosystem directions, 13 candidate fields. These are expansion ideas, not currently supported integrations; availability depends on provider interfaces.','.ecosystem-copy .why-link':'Explore current services ↘'};
  for(const [selector,value] of Object.entries(ecosystemCopy)){const node=document.querySelector(selector);if(node)node.textContent=value}
  const domains=[['AI & content creation','Models & agents · Images & video · Voice & translation · Search & crawling'],['Productivity & business','Workflow automation · Office & design SaaS · Commerce & logistics · Marketing & communication'],['Digital resources & infrastructure','Cloud & compute · Databases & analytics · Storage & backups · Networks & traffic'],['Personal digital memberships','Consumer tools, knowledge & education memberships']];
  document.querySelectorAll('.ecosystem-domains>div').forEach((node,i)=>{node.querySelector('span').textContent=domains[i][0];node.querySelector('p').textContent=domains[i][1]});
  setTexts('.agent-portrait span, .note-kicker', ['live','portraitLabel']); setText('.blueprint-note p', 'portraitCopy', true); setText('.note-foot', 'portraitNote'); setText('.stage-rail>span','workflow'); setText('.keyboard-demo>span','keyboardKicker'); setText('.config-kicker','runtimeKicker');
  setText('.download-section .section-index','downloadIndex'); setText('.demo-section .section-head h2', 'demoTitle'); setText('.demo-section .section-head p', 'demoDesc', true); setText('.demo-status .chip__label', 'demoStatus'); setText('#reset-demo', 'reset'); document.querySelector('.workflow-guide summary').textContent='Desktop keyboard shortcuts'; document.querySelector('.ecosystem-roadmap summary').textContent='Future integration directions';
  setTexts('.shortcut-list span', ['alt2','shift','ctrls','esc']);
  setTexts('.keyboard-demo button b', ['toggle','close','period','category','settingsShortcut','customize','themeShortcut','refreshShortcut','expiring']); setTexts('.stage-config dt', ['runtime','runtimeSettings','agentSelf']); setTexts('.stage-config dd', ['runtimeCopy','runtimeSettingsCopy','agentSelfCopy']);
  const notes=[['.annotation-sample strong','annSample'],['.annotation-sample>span:not(.annotation-number)','annSampleCopy'],['.annotation-left strong','annEdge'],['.annotation-left>span','annEdgeCopy'],['.annotation-right strong','annProviders'],['.annotation-right>span','annProvidersCopy'],['.annotation-tabs strong','annTabs'],['.annotation-tabs>span:not(.annotation-number)','annTabsCopy'],['.annotation-periods strong','annPeriods'],['.annotation-periods>span:not(.annotation-number)','annPeriodsCopy'],['.annotation-accounts strong','annAccounts'],['.annotation-accounts>span:not(.annotation-number)','annAccountsCopy'],['.annotation-spend strong','annSpend'],['.annotation-spend>span:not(.annotation-number)','annSpendCopy'],['.annotation-pool strong','annPool'],['.annotation-pool>span','annPoolCopy'],['.annotation-groups strong','annGroups'],['.annotation-groups>span:not(.annotation-number)','annGroupsCopy'],['.annotation-status strong','annStatus'],['.annotation-status>span','annStatusCopy'],['.annotation-settings strong','annSettings'],['.annotation-settings>span:not(.annotation-number)','annSettingsCopy'],['.annotation-refresh strong','annRefresh'],['.annotation-refresh>span','annRefreshCopy']]; notes.forEach(([s,k])=>setText(s,k,k.endsWith('Copy')));
  setTexts('.demo-controls button span', ['overview','spend','settings','skins','refresh']); setText('.demo-rest>b','restTitle'); setText('.demo-rest p','restCopy'); setText('.keyboard-demo>span','keyboardKicker'); setText('.keyboard-demo h3','keyboardTitle'); setText('.keyboard-demo>p','keyboardCopy'); setText('#keyboard-start','start'); setText('.keyboard-feedback__text','keyboardNote'); setText('.keyboard-demo>small','keyboardHint');
  setText('.provider-section .section-head h2','providersTitle'); setText('.provider-section .section-head p','providersCopy',true); setText('.feature-section .section-head h2','featuresTitle'); setText('.feature-section .section-head p','featuresCopy'); setTexts('.feature-grid h3',['quotaTitle','spendTitle','resetTitle','privacyTitle']); setTexts('.feature-grid p',['quotaCopy','spendCopy','resetCopy','privacyCopy']);
  setTexts('.frag-title',['fragQuota','fragSpend','fragReset','fragKeys']); setTexts('.frag-tag .chip__label',['fragTagQuota','fragTagSpend','fragTagReset','fragTagKeys']); setTexts('.frag-row b',['fragRow5h','fragRowWeek','fragRowMonth']); setText('.frag-total span','fragToday'); setText('.frag-note','fragResetNote');   // FAQ English copy resolves through the group/item structure (group 1 items
  // 1-3 -> faq.q1..q3, and so on) rather than one flat positional list, so the
  // key for each row is derived from its place in the groups instead of from a
  // hand-maintained array that regrouping would silently misalign.
  document.querySelectorAll('.faq-group').forEach((group, gi) => { const set = (node, key) => { if (node && isEnglish && en[key]) node.textContent = en[key]; }; set(group.querySelector('.faq-group-title'), `faq.g${gi + 1}`); group.querySelectorAll('.faq-item').forEach((item, ii) => { const n = gi * 3 + ii + 1; set(item.querySelector('summary'), `faq.q${n}`); set(item.querySelector('.faq-body p'), `faq.a${n}`); }); });
  setText('.faq-section .section-head p', 'faqCopy');
  setText('.faq-contact span', 'faq.contactPrompt'); setText('.faq-contact a', 'faq.cta');
  setTexts('.bottom-actions .chip__label',['coming','coming']); setText('.bottom-actions .button[data-variant=secondary]','tryDemo'); setText('.platforms','downloadNote'); setText('[data-release-prefix]','releasePrefix'); setTexts('.site-footer .footer-content>div a',['source','docs','feedback']);
  setText('#design .section-index','designIndex'); setText('#design .section-head h2','designTitle'); setText('#design .section-head p','designCopy'); setTexts('#design .design-card h3',['designCard1','designCard2']); setTexts('#design .design-card p',['designCard1Copy','designCard2Copy']); setTexts('#design .design-open',['designOpen','designOpen']);
  setText('.signal-section .section-index','signalsIndex'); setText('#signals-title','signalsTitle'); setText('.signal-section .section-head p','signalsCopy');
  setText('.signal-beam .signal-card-tag','sigRefreshTag'); setText('.signal-beam .signal-caption h3','sigRefreshTitle'); setText('.signal-beam .signal-caption p','sigRefreshCopy'); setTexts('.signal-beam .sig-node>span',['sigNodeTimer','sigNodeManual','sigNodeLaunch','sigNodeSnapshot','sigNodePanel']); setText('.signal-beam .sig-node--hub b','sigNodeGate');
  setText('.signal-metric .signal-card-tag','sigMetricTag'); setText('.signal-metric .signal-card-alt','sigSample'); setText('.signal-metric .signal-caption h3','sigMetricTitle'); setText('.signal-metric .signal-caption p','sigMetricCopy'); setText('.sig-metric-label','sigMetricLabel'); setText('.sig-metric-delta>span','sigMetricMedian');
  setText('.signal-terminal .signal-card-tag','sigTermTag'); setText('.signal-terminal .signal-caption h3','sigTermTitle'); setText('.signal-terminal .signal-caption p','sigTermCopy'); setText('.sig-term-note','sigTermNote');
  setText('.signal-keys .signal-card-tag','sigKeysTag'); setText('.signal-keys .signal-card-alt','sigSystemHotkey'); setText('.signal-keys .signal-caption h3','sigKeysTitle'); setText('.signal-keys .signal-caption p','sigKeysCopy'); setText('.sig-command b','sigKeysPanel');
}
applyEnglishCopy();
initUsageFlow(isEnglish,en);
initProviderOrbits();
initProviderMarquee();
initDataDisplay();

// Headline text reveal (#86). CSS owns the end state; this only flips the
// class when the heading enters the viewport. A missing IntersectionObserver
// reveals everything immediately instead of leaving a blurred heading.
document.querySelectorAll('.reveal-text').forEach(node=>{
  if(!('IntersectionObserver' in window)){node.classList.add('is-revealed');return}
  const observer=new IntersectionObserver(entries=>{
    if(!entries[0].isIntersecting)return;
    observer.disconnect();node.classList.add('is-revealed');
  },{threshold:.25});
  observer.observe(node);
});

// The provider belt is the one self-moving surface that had no switch of its
// own; it now shares the page-wide one, which is also what makes the belt
// comply with WCAG 2.2.2 (hover-pause alone is not a pause mechanism).
function initProviderMarquee(){
  const integrations=document.querySelector('.integrations');
  if(!integrations)return;
  paneMotion.subscribe(()=>integrations.classList.toggle('is-paused',paneMotion.paused));
  bindMotionToggle(document.querySelector('.marquee-pause'),paneMotion,motionLabel);
}

// Numbers and meter semantics — the part of the HeroUI model that survives
// without React. Every figure the page shows used to be a hard-coded string;
// they now carry the number plus its shape and are rendered through Intl, and
// the five source rows get a real accessible name built from their own parts
// instead of the run-on flattened text a button would otherwise announce.
function initDataDisplay(){
  const reduced=window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  for(const node of document.querySelectorAll('[data-number]')){
    const value=Number(node.dataset.number);
    if(!Number.isFinite(value))continue;
    const final=formatByKind(node.dataset.format,value)+(node.dataset.unit||'');
    node.textContent=final;
    if(reduced||value===0)continue;
    // One 1.4s count-up per figure, started when the figure is actually on
    // screen. The final string is already in place for assistive tech and for
    // a page whose observer never fires; only sighted viewers see the ticks.
    const format=(v)=>formatByKind(node.dataset.format,v)+(node.dataset.unit||'');
    let started=false;
    const start=()=>{
      if(started)return;
      started=true;
      const began=performance.now();
      const tick=(now)=>{
        const t=Math.min(1,(now-began)/1400);
        const eased=1-Math.pow(1-t,3);
        node.textContent=t<1?format(value*eased):final;
        if(t<1)requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    };
    if('IntersectionObserver' in window){
      const io=new IntersectionObserver(entries=>{
        if(!entries[0].isIntersecting)return;
        io.disconnect();start();
      },{threshold:.45});
      io.observe(node);
    }else start();
  }
  const joiner=isEnglish?', ':'，';
  for(const row of document.querySelectorAll('.why-source')){
    const name=row.querySelector('b')?.childNodes[0]?.textContent.trim();
    const source=row.querySelector('small')?.textContent.trim();
    const value=row.querySelector('em')?.textContent.trim();
    const label=[name,source,value].filter(Boolean).join(joiner);
    if(label)row.setAttribute('aria-label',label);
  }
  // The three hero stats used to read as one run-on blob; each gets its own name.
  for(const stat of document.querySelectorAll('.hero-stats>div')){
    const value=stat.querySelector('b')?.textContent.trim();
    const label=stat.querySelector('span')?.textContent.trim();
    if(value&&label)stat.setAttribute('aria-label',`${value} ${label}`);
  }
  if(isEnglish)document.querySelector('.frag-chart')?.setAttribute('aria-label',en.fragChartLabel);
}

function initProviderOrbits(){
  const section=document.querySelector('.ecosystem-section');if(!section)return;
  const visual=section.querySelector('.ecosystem-visual');
  const button=section.querySelector('.ecosystem-pause');
  const card=section.querySelector('.orbit-card');
  const icons=[...section.querySelectorAll('.why-orbit-icon')];
  if(!visual||!button)return;
  let visible=!('IntersectionObserver' in window);
  const sync=()=>section.classList.toggle('is-paused',paneMotion.paused||!visible||document.hidden);
  // The orbit switch is the page-wide switch, not a private one: it carries the
  // same label as the collector's and moves every ambient animation together.
  bindMotionToggle(button,paneMotion,motionLabel);
  paneMotion.subscribe(sync);
  if('IntersectionObserver' in window)new IntersectionObserver(entries=>{visible=entries[0].isIntersecting;sync()},{threshold:.05}).observe(section);
  document.addEventListener('visibilitychange',sync);sync();

  // Hover card. Its labels come from copy the section already owns — the four
  // directions and the page's general promise — so no per-provider capability
  // is asserted that the site cannot back up.
  const directions={1:['AI 与内容生产','AI & content creation'],2:['工作效率与商业运营','Productivity & business'],3:['数字资源与开发基础设施','Digital resources & infrastructure'],4:['个人数字会员','Personal memberships']};
  const cardNote=isEnglish?'Quota, balance and subscription status in one floating window.':'额度 · 余额 · 订阅状态，同一个悬浮窗。';
  const cardCta=isEnglish?'See connected services ↘':'查看已连接服务 ↘';
  // The card follows React Aria's PreviewTrigger model — which is what HeroUI's
  // hover card wraps: an open delay that is skipped inside the shared warm
  // window, a close delay long enough to travel into the card, and keyboard +
  // long-press paths, because hover exists for none of those visitors.
  const openCard=icon=>{
    if(!card)return;
    const info=directions[icon.dataset.domain];
    const name=icon.getAttribute('aria-label')||'';
    card.style.setProperty('--domain',`var(--domain-${icon.dataset.domain})`);
    card.querySelector('.orbit-card-name').textContent=name;
    card.querySelector('.orbit-card-domain').textContent=info?info[isEnglish?1:0]:'';
    card.querySelector('.orbit-card-note').textContent=cardNote;
    card.querySelector('.orbit-card-cta').textContent=cardCta;
    card.setAttribute('aria-label',name);
    card.hidden=false;
    // Sit beside the icon. Hovering the orbit freezes the icons, so the card
    // does not have to chase a moving target.
    const box=visual.getBoundingClientRect(),r=icon.getBoundingClientRect();
    const left=Math.max(6,Math.min(r.right-box.left+12,box.width-card.offsetWidth-6));
    const top=Math.max(6,Math.min(r.top-box.top+r.height/2-card.offsetHeight/2,box.height-card.offsetHeight-6));
    card.style.left=`${left}px`;card.style.top=`${top}px`;
    warmHover();
  };
  const closeCard=()=>{if(card)card.hidden=true};
  let openTimer=0,closeTimer=0,openIcon=null;
  const cancelTimers=()=>{clearTimeout(openTimer);clearTimeout(closeTimer)};
  const scheduleOpen=icon=>{cancelTimers();openIcon=icon;openTimer=setTimeout(()=>openCard(icon),isHoverWarm()?0:180)};
  // The close is delayed and cancelled when the pointer reaches the card, so
  // travelling from the icon into the card never dismisses it.
  const scheduleClose=()=>{clearTimeout(closeTimer);closeTimer=setTimeout(closeCard,300)};
  card?.addEventListener('pointerenter',cancelTimers);
  card?.addEventListener('pointerleave',scheduleClose);
  card?.addEventListener('keydown',event=>{if(event.key==='Escape'){cancelTimers();closeCard();openIcon?.focus()}});
  icons.forEach(icon=>{
    icon.addEventListener('pointerenter',()=>scheduleOpen(icon));
    icon.addEventListener('pointerleave',scheduleClose);
    icon.addEventListener('focus',()=>{cancelTimers();openCard(icon)});
    icon.addEventListener('blur',()=>{if(!card?.contains(document.activeElement))scheduleClose()});
    icon.addEventListener('keydown',event=>{if(event.key==='Escape'){cancelTimers();closeCard();icon.focus()}});
    onLongPress(icon,()=>openCard(icon));
    // Clicking a provider jumps to the list of everything Pane already reads.
    icon.addEventListener('click',()=>{location.hash='#connected-providers'});
  });
  // Escape closes from anywhere, unless something closer already handled it
  // (the keyboard demo's own Escape handler marks the event as handled).
  document.addEventListener('keydown',event=>{
    if(event.defaultPrevented||event.key!=='Escape'||!card||card.hidden)return;
    cancelTimers();closeCard();openIcon?.focus();
  });
  // Pointing at a direction in the right-hand list lights its icons on the left.
  section.querySelectorAll('.ecosystem-domains>div').forEach((row,i)=>{
    const value=String(i+1);
    row.addEventListener('pointerenter',()=>{visual.dataset.domainFocus=value});
    row.addEventListener('pointerleave',()=>{delete visual.dataset.domainFocus});
  });
}
document.querySelector('#language')?.addEventListener('click',()=>{location.href=isEnglish?'/':'/en/'});

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
  if(event.source===settingsFrame.contentWindow&&event.origin===location.origin&&event.data?.type==='pane-demo'&&event.data.action==='tour-replay'){
    setSettingsOverlay(false);send('tour-replay');return;
  }
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
// Hero ink marks: the hand-drawn highlighter treatment. rough-notation is
// the library Magic UI's Highlighter component wraps; this site is
// zero-framework, so the annotate calls live here with the demo's colours
// (highlight #87CEFA / underline #FF9800). Marks are drawn only after the
// CSS entrance settles — rough-notation measures the element's box, and
// animating children would skew the SVG it injects.
const inkMarks=[...document.querySelectorAll('.ink-mark')];
const reduceMotion=matchMedia('(prefers-reduced-motion: reduce)').matches;
function drawInkMarks(){
  for(const el of inkMarks){
    el.__rough?.remove();
    const mark=RoughNotation.annotate(el,{
      type:el.dataset.hlType||'highlight',
      color:el.dataset.hlColor||'#87CEFA',
      strokeWidth:Number(el.dataset.hlStroke||1.5),
      animationDuration:reduceMotion?0:600,
      iterations:2,
      padding:Number(el.dataset.hlPadding||2),
      multiline:true,
    });
    mark.show();
    el.__rough=mark;
  }
}
let inkStarted=false;
function initInkMarks(){
  if(!inkMarks.length||typeof RoughNotation==='undefined')return;
  const start=()=>{
    if(inkStarted)return;
    inkStarted=true;
    drawInkMarks();
    // Webfonts swap in after first paint and change the measured boxes.
    document.fonts?.ready.then(drawInkMarks);
    // Redraw on layout shifts, mirroring the upstream component's observers.
    const ro=new ResizeObserver(()=>drawInkMarks());
    for(const el of inkMarks)ro.observe(el);
    ro.observe(document.body);
  };
  if(reduceMotion){start();return}
  // .hero-title is a .hero-copy child, so it carries the rise-in animation;
  // its animationend fires once the entrance has finished.
  document.querySelector('.hero-title')?.addEventListener('animationend',start,{once:true});
  setTimeout(start,1800); // backgrounded tabs throttle animation events
}
initInkMarks();
const settingsButton=document.querySelector('[data-demo=settings]');const settingsWrap=document.querySelector('#pane-settings-wrap');let settingsOpener=null;
// A real modal. role/aria-modal live on the markup; the script owns what the
// markup cannot — moving focus in on open, trapping Tab inside, closing on
// Escape, and handing focus back to whatever opened it.
function setSettingsOverlay(open){
  const wasHidden=settingsWrap.hidden;
  if(open&&wasHidden){
    if(!settingsFrame.dataset.loaded){settingsFrame.src=`/demo/index.html?lang=${demoLang}&theme=${demoTheme}&settings=1`;settingsFrame.dataset.loaded='1'}
    settingsWrap.hidden=false;
    settingsOpener=document.activeElement;
    document.querySelector('#pane-settings-close').focus();
  }
  if(!open&&!wasHidden){
    settingsWrap.hidden=true;
    (settingsOpener instanceof HTMLElement?settingsOpener:settingsButton)?.focus();
  }
  settingsButton.classList.toggle('active',!settingsWrap.hidden);
  settingsButton.setAttribute('aria-expanded',String(!settingsWrap.hidden));
}
settingsWrap.addEventListener('keydown',event=>{
  if(event.key==='Escape'){event.preventDefault();setSettingsOverlay(false);return}
  if(event.key!=='Tab')return;
  const focusables=[...settingsWrap.querySelectorAll('button,[href],iframe,[tabindex]:not([tabindex="-1"])')].filter(el=>!el.hasAttribute('disabled'));
  if(!focusables.length)return;
  const first=focusables[0],last=focusables[focusables.length-1];
  if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus()}
  else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus()}
});settingsButton.addEventListener('click',()=>setSettingsOverlay(settingsWrap.hidden));document.querySelector('#pane-settings-close').addEventListener('click',()=>setSettingsOverlay(false));function send(action){frame.contentWindow?.postMessage({type:'pane-demo',action},location.origin)}function loadDemo(){frame.src=`/demo/index.html?lang=${demoLang}&theme=${demoTheme}`;settingsWrap.hidden=true;settingsFrame.removeAttribute('src');delete settingsFrame.dataset.loaded;selected='overview';desktop.dataset.mode='auto';syncDemoControls()}
// The iframe is the page's heaviest request; keep it out of first paint and
// load it once the demo section is within 700px. loading="lazy" alone was not
// enough — this script called loadDemo() immediately, so the browser fetched
// the whole demo before the hero had painted.
let demoBooted=false;
function bootDemo(){if(demoBooted)return;demoBooted=true;loadDemo()}
const demoSection=document.querySelector('#demo');
if(demoSection&&'IntersectionObserver' in window){
  const demoBoot=new IntersectionObserver(entries=>{
    if(!entries[0].isIntersecting)return;
    demoBoot.disconnect();bootDemo();
  },{rootMargin:'700px 0px'});
  demoBoot.observe(demoSection);
}else bootDemo();
document.querySelector('#reset-demo').addEventListener('click',loadDemo);document.querySelectorAll('[data-demo]').forEach(b=>b.addEventListener('click',()=>{const a=b.dataset.demo;if(a==='settings'){desktop.dataset.mode='settings';updateNoteAct();return}if(a!=='refresh')setSettingsOverlay(false);if(a==='refresh'){desktop.dataset.mode='refresh';updateNoteAct();b.disabled=true;send(a);clearTimeout(refreshTimer);refreshTimer=setTimeout(()=>b.disabled=false,650);return}selected=a;desktop.dataset.mode=a;send(a);syncDemoControls();updateNoteAct()}));// One panel open per FAQ group. `<details>` stays because it is the more
// accessible primitive — the FAQ keeps working with no script at all — so the
// accordion keeps its disclosure list and only gains what the primitive lacks.
document.querySelectorAll('.faq-group').forEach(group=>{
  const items=[...group.querySelectorAll('details.faq-item')];
  items.forEach(item=>item.addEventListener('toggle',()=>{if(!item.open)return;items.forEach(other=>{if(other!==item)other.open=false})}));
});
// HeroUI's accordion lets the arrow keys walk a group's headers. Tab still
// leaves the group, so this is an addition, not a different model.
function initFaqNavigation(){
  for(const group of document.querySelectorAll('.faq-group')){
    const summaries=[...group.querySelectorAll('.faq-item>summary')];
    if(!summaries.length)continue;
    summaries.forEach(summary=>summary.addEventListener('keydown',event=>{
      if(event.key==='Home'){event.preventDefault();summaries[0].focus();return}
      if(event.key==='End'){event.preventDefault();summaries[summaries.length-1].focus();return}
      moveRovingFocus(summaries,summary,event,{orientation:'vertical'});
    }));
  }
}
// The four demo controls are a toolbar, not a segmented control: two switch the
// panel's view, one opens the settings overlay, one fires a refresh. So the
// group carries toolbar semantics with roving focus, and each control carries
// the state it actually has (pressed / expanded).
function syncDemoControls(){
  document.querySelectorAll('[data-demo]').forEach(button=>{
    const on=button.dataset.demo===selected;
    button.classList.toggle('active',on);
    if(button.hasAttribute('aria-pressed'))button.setAttribute('aria-pressed',String(on));
  });
}
function initDemoToolbar(){
  const toolbar=document.querySelector('.demo-controls');
  if(!toolbar)return;
  const buttons=[...toolbar.querySelectorAll('button')];
  toolbar.addEventListener('keydown',event=>{
    if(event.key==='Home'){event.preventDefault();buttons[0].focus();return}
    if(event.key==='End'){event.preventDefault();buttons[buttons.length-1].focus();return}
    moveRovingFocus(buttons,document.activeElement,event);
  });
}
initFaqNavigation();
initDemoToolbar();
syncDemoControls();

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
        // The duplicate belt is a visual device. Marking it aria-hidden keeps
        // assistive tech from reading every provider twice.
        [...track.children].forEach(item=>{const clone=item.cloneNode(true);clone.setAttribute('aria-hidden','true');track.append(clone)});
        track.dataset.looped='true';
      }
      track.classList.add(index ? 'belt-reverse' : 'belt-forward');
    });
    // Feature fragments. The entrance builds each cell once — quota bars fill,
    // the spend chart grows, the credential rows slide in — and then leaves one
    // quiet idle beat per cell. This replaces the old infinite yoyo that ran on
    // every element of all four cells at the same time; the cells now borrow the
    // panel's calm instead of pulsing at the visitor. Idles only run while the
    // grid is on screen and the shared switch says motion is allowed.
    document.querySelectorAll('.feature-art .frag').forEach(frag => {
      const fills = frag.querySelectorAll('.frag-row em');
      const track = frag.querySelector('.frag-track i');
      const bars = frag.querySelectorAll('.frag-chart i');
      const keys = frag.querySelectorAll('.frag-key');
      if (fills.length) gsap.fromTo(fills, { scaleX: 0 }, { scaleX: 1, duration: .85, stagger: .1, ease: 'power3.out', transformOrigin: 'left center' });
      if (track) gsap.fromTo(track, { scaleX: 0 }, { scaleX: 1, duration: 1, ease: 'power3.out', transformOrigin: 'left center' });
      if (bars.length) gsap.fromTo(bars, { scaleY: 0 }, { scaleY: 1, duration: .7, stagger: .06, ease: 'power3.out', transformOrigin: 'bottom center' });
      if (keys.length) gsap.fromTo(keys, { opacity: 0, x: -6 }, { opacity: 1, x: 0, duration: .5, stagger: .12, ease: 'power2.out' });
    });
    const fragIdles = [];
    document.querySelectorAll('.feature-art .frag').forEach(frag => {
      const fills = frag.querySelectorAll('.frag-row em');
      const peak = frag.querySelector('.frag-chart i:last-child');
      const masked = frag.querySelectorAll('.frag-key span');
      if (fills.length) fragIdles.push(gsap.to(fills, { opacity: .72, duration: 1.9, yoyo: true, repeat: -1, ease: 'sine.inOut', stagger: .25 }));
      if (peak) fragIdles.push(gsap.to(peak, { scaleY: 1.06, duration: 1.7, yoyo: true, repeat: -1, ease: 'sine.inOut', transformOrigin: 'bottom center' }));
      if (masked.length) fragIdles.push(gsap.to(masked, { opacity: .55, duration: 2.2, yoyo: true, repeat: -1, ease: 'sine.inOut', stagger: .3 }));
    });
    let fragOnScreen = false;
    const syncFragIdles = () => {
      const run = fragOnScreen && !paneMotion.paused && !document.hidden;
      for (const tween of fragIdles) run ? tween.resume() : tween.pause();
    };
    for (const tween of fragIdles) tween.pause();
    const gridEl = document.querySelector('.feature-grid');
    if (gridEl && 'IntersectionObserver' in window) new IntersectionObserver(entries => { fragOnScreen = entries[0].isIntersecting; syncFragIdles(); }, { threshold: .1 }).observe(gridEl);
    else fragOnScreen = true;
    document.addEventListener('visibilitychange', syncFragIdles);
    paneMotion.subscribe(syncFragIdles);
    syncFragIdles();
    // Signal cards enter once, when the section reaches the viewport. The beam
    // keeps its CSS timeline (finite) and this block only adds the class that
    // starts it, so a throttled tab still renders every card in its end state.
    const signalSection = document.querySelector('.signal-section');
    if (signalSection) {
      const signalReveal = new IntersectionObserver(entries => {
        if (!entries[0].isIntersecting) return;
        signalReveal.disconnect();
        signalSection.classList.add('is-live');
        gsap.timeline({ defaults: { ease: 'power3.out' } })
          .fromTo(signalSection.querySelectorAll('.sig-node'), { opacity: 0 }, { opacity: 1, duration: .35, stagger: .05 }, 0)
          .fromTo(signalSection.querySelectorAll('.sig-metric-chart i'), { scaleY: 0 }, { scaleY: 1, duration: .6, stagger: .022, transformOrigin: 'bottom center' }, .05)
          .fromTo(signalSection.querySelectorAll('.sig-metric-value, .sig-metric-delta'), { opacity: 0, y: 6 }, { opacity: 1, y: 0, duration: .45, stagger: .08 }, .15)
          .fromTo(signalSection.querySelectorAll('.sig-term-line'), { opacity: 0, y: 5 }, { opacity: 1, y: 0, duration: .24, stagger: .09 }, .2)
          .fromTo(signalSection.querySelectorAll('.sig-keycap, .sig-command'), { opacity: 0 }, { opacity: 1, duration: .35, stagger: .1 }, .25);
      }, { threshold: .18 });
      signalReveal.observe(signalSection);
    }
    // Section indices pop in once, with a small overshoot. They are the page's
    // only numbering system, so the motion points at the structure rather
    // than decorating every chip on the page.
    const indexPop = new IntersectionObserver(entries => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        indexPop.unobserve(entry.target);
        gsap.fromTo(entry.target, { scale: .72, opacity: 0 }, { scale: 1, opacity: 1, duration: .5, ease: 'back.out(2)' });
      }
    }, { threshold: .6 });
    document.querySelectorAll('.section-index').forEach(node => indexPop.observe(node));
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
  // The status line is a toast, not a paragraph: a status colour plus an
  // indicator, and aria-atomic so the whole message is announced instead of
  // only the fragment that changed.
  function setFeedback(text,status='info'){
    feedback.dataset.status=status;
    const node=feedback.querySelector('.keyboard-feedback__text');
    if(node)node.textContent=text;else feedback.textContent=text;
  }
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
    setFeedback(note || (active ? (visible ? (isEnglish ? `Panel visible · ${periods[period]}` : `面板已浮现 · 当前 ${periods[period]}`) : (isEnglish ? 'Panel dismissed · press Alt + 2 or use a key above' : '面板已收起 · 按 Alt + 2 或上方按键唤出')) : (isEnglish ? 'Start here, then try a shortcut.' : '点击开始，再试试快捷键。')));
    positionAnnotations();
  }
  function say(text) {
    if (!active) return;
    note = text;
    setFeedback(text,'success');
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

// Global animated grid background: Magic UI's Animated Grid Pattern
// (upstream source archived in vendor/magicui-animated-grid-pattern/) driven
// by the site's vendored GSAP instead of React + motion. Same DOM shape (SVG
// pattern + a rect grid), same 40px cells, same animation parameters —
// duration / repeat:1 with yoyo (= motion's repeatType:"reverse") /
// repeatDelay / delay:index*0.1 / random reposition on complete. Colours live
// in CSS theme variables, so flipping the theme needs no re-init. Idempotent,
// and degrades to the static grid when GSAP is absent.
function initGridBackground(){
  const host=document.querySelector('#grid-bg');
  if(!host||host.dataset.ready)return;
  host.dataset.ready='1';
  const cell=Number(host.dataset.gridSize)||40;
  const count=Number(host.dataset.gridSquares)||30;
  const maxOpacity=Number(host.dataset.gridMaxOpacity)||0.1;
  const duration=Number(host.dataset.gridDuration)||3;
  const repeatDelay=Number(host.dataset.gridRepeatDelay)||1;
  const ns='http://www.w3.org/2000/svg';
  const patternId=`grid-bg-${Math.random().toString(36).slice(2,9)}`;
  host.innerHTML=`<svg aria-hidden="true"><defs><pattern id="${patternId}" width="${cell}" height="${cell}" patternUnits="userSpaceOnUse" x="-1" y="-1"><path class="grid-line" d="M.5 ${cell}V.5H${cell}"/></pattern></defs><rect width="100%" height="100%" fill="url(#${patternId})"/><svg x="-1" y="-1"><g class="grid-squares"></g></svg></svg>`;
  const layer=host.querySelector('.grid-squares');
  const reduceMotion=matchMedia('(prefers-reduced-motion: reduce)').matches;
  let dims={width:0,height:0}, ctx=null, ro=null, tweens=[];
  const gridW=()=>Math.max(1,Math.floor(dims.width/cell));
  const gridH=()=>Math.max(1,Math.floor(dims.height/cell));
  function place(rect){
    rect.setAttribute('x',Math.floor(Math.random()*gridW())*cell+1);
    rect.setAttribute('y',Math.floor(Math.random()*gridH())*cell+1);
  }
  // The breathing squares answer to the page-wide switch like every other
  // ambient animation. They are the one effect a visitor cannot reach from the
  // section they live in, so the shared state has to arrive from here.
  function applyMotion(){
    const run=!paneMotion.paused&&!document.hidden;
    for(const tween of tweens)if(tween)run?tween.resume():tween.pause();
  }
  function build(){
    ctx?.revert();
    ctx=null;
    tweens=[];
    layer.textContent='';
    // Reduced motion, or a phone-width viewport: keep the static grid only.
    if(reduceMotion||dims.width<=720||!window.gsap)return;
    ctx=gsap.context(()=>{
      for(let i=0;i<count;i++){
        const rect=document.createElementNS(ns,'rect');
        rect.setAttribute('class','grid-square');
        rect.setAttribute('width',cell-1);
        rect.setAttribute('height',cell-1);
        rect.setAttribute('opacity','0');
        layer.appendChild(rect);
        // A wider stagger than the upstream 0.1s: with fewer squares each pulse
        // should read as one deliberate mark, not a drifting blob.
        const run=()=>{
          place(rect);
          tweens[i]=gsap.fromTo(rect,{opacity:0},{opacity:maxOpacity,duration,repeat:1,yoyo:true,repeatDelay,delay:i*0.22,onComplete:run});
        };
        run();
      }
    });
    applyMotion();
  }
  const measure=()=>{
    const w=Math.round(host.clientWidth), h=Math.round(host.clientHeight);
    if(!w||!h||(w===dims.width&&h===dims.height))return;
    dims={width:w,height:h};
    build();
  };
  measure();
  if('ResizeObserver' in window){ro=new ResizeObserver(measure);ro.observe(host)}
  paneMotion.subscribe(applyMotion);
  document.addEventListener('visibilitychange',applyMotion);
  window.addEventListener('pagehide',()=>{ro?.disconnect();ctx?.revert()},{once:true});
}
initGridBackground();

initHeaderNav();
initHeroReactions();
initLineSidebar();
const siteEffects = initSiteEffects();
initThemeSwap({ root, button:document.querySelector('#theme'), syncEffects:() => siteEffects?.syncTheme(), motionPaused:() => paneMotion.paused });

// The reset cell's countdown is the one part of the feature row that keeps
// moving on its own: a real clock, so the cell reads as the live panel rather
// than a screenshot of it. It honours the shared switch and reduced motion.
function initFeatureCountdown(){
  const el=document.querySelector('.frag-countdown');
  if(!el)return;
  const reduce=matchMedia('(prefers-reduced-motion: reduce)');
  const start=Number(el.dataset.countdown)||0;
  let left=start;
  const pad=n=>String(n).padStart(2,'0');
  const paint=()=>{el.textContent=`${pad(Math.floor(left/3600))}:${pad(Math.floor(left/60)%60)}:${pad(left%60)}`};
  paint();
  setInterval(()=>{
    if(paneMotion.paused||reduce.matches||document.hidden)return;
    left=left>0?left-1:start;
    paint();
  },1000);
}
initFeatureCountdown();
