// MOTDES LineSidebar's smooth proximity curve and exponential rAF easing,
// adapted to real anchor links, scroll position and Pane's shared pause state.
import { paneMotion } from '/motion.js?v=2';

export function initLineSidebar() {
  const english = document.documentElement.lang === 'en';
  const entries = [
    ['overview', '概览', 'Overview'], ['why-pane', '为何选择', 'Why Pane'],
    ['demo', '互动演示', 'Demo'], ['ecosystem', '连接生态', 'Ecosystem'],
    ['connected-providers', '已连接服务', 'Providers'], ['features', '核心能力', 'Features'],
    ['design', '架构与数据', 'Architecture'], ['faq', '常见问题', 'FAQ'], ['download', '下载', 'Download'],
  ].filter(([id]) => document.getElementById(id));
  const nav = document.createElement('nav');
  nav.id = 'section-directory';
  nav.className = 'line-sidebar';
  nav.setAttribute('aria-label', english ? 'On this page' : '本页目录');
  nav.innerHTML = `<ol class="line-sidebar__list">${entries.map(([id, zh, en], index) => `<li class="line-sidebar__item"><span class="line-sidebar__marker" aria-hidden="true"></span><a href="#${id}"><span class="line-sidebar__index" aria-hidden="true">${String(index + 1).padStart(2, '0')}</span><span>${english ? en : zh}</span></a></li>`).join('')}</ol>`;
  const toggle = document.createElement('button');
  toggle.type = 'button';
  toggle.className = 'line-sidebar-toggle';
  toggle.textContent = english ? '☰ Contents' : '☰ 目录';
  toggle.setAttribute('aria-controls', nav.id);
  toggle.setAttribute('aria-expanded', 'false');
  document.body.append(nav, toggle);
  const rows = [...nav.querySelectorAll('li')];
  const links = rows.map(row => row.querySelector('a'));
  const sections = entries.map(([id]) => document.getElementById(id));
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const proximity = rows.map(() => 0), current = rows.map(() => 0);
  let active = 0, raf = 0, scrollFrame = 0, last = 0;
  const animate = () => !paneMotion.paused && !reduced.matches && !document.hidden && !document.documentElement.hasAttribute('data-theme-transition');
  const paint = now => {
    raf = 0;
    const dt = Math.max(0, Math.min((now - last) / 1000, .05));
    last = now;
    const k = animate() ? 1 - Math.exp(-dt / .1) : 1;
    let moving = false;
    rows.forEach((row, i) => {
      const target = Math.max(proximity[i], i === active ? 1 : 0);
      const next = current[i] + (target - current[i]) * k;
      current[i] = Math.abs(target - next) < .0015 ? target : next;
      row.style.setProperty('--effect', current[i].toFixed(4));
      moving ||= current[i] !== target;
    });
    if (moving && animate()) raf = requestAnimationFrame(paint);
  };
  const wake = () => {
    if (raf) cancelAnimationFrame(raf);
    last = performance.now();
    if (animate()) raf = requestAnimationFrame(paint);
    else paint(last);
  };
  const setActive = index => {
    active = index;
    links.forEach((link, i) => i === active ? link.setAttribute('aria-current', 'location') : link.removeAttribute('aria-current'));
    wake();
  };
  const track = () => {
    scrollFrame = 0;
    let next = 0;
    sections.forEach((section, i) => { if (section.getBoundingClientRect().top <= Math.max(100, innerHeight * .3)) next = i; });
    if (next !== active) setActive(next);
  };
  const onScroll = () => { if (!scrollFrame) scrollFrame = requestAnimationFrame(track); };
  const clearProximity = () => { proximity.fill(0); wake(); };
  const onMove = event => {
    if (event.pointerType !== 'mouse' || !animate()) return;
    rows.forEach((row, i) => {
      const box = row.getBoundingClientRect();
      const p = Math.max(0, 1 - Math.abs(event.clientY - box.top - box.height / 2) / 88);
      proximity[i] = p * p * (3 - 2 * p);
    });
    wake();
  };
  const close = () => { if (nav.dataset.open !== 'true') return; nav.dataset.open = 'false'; toggle.setAttribute('aria-expanded', 'false'); clearProximity(); };
  toggle.addEventListener('click', () => {
    const open = nav.dataset.open !== 'true';
    nav.dataset.open = String(open); toggle.setAttribute('aria-expanded', String(open));
    if (open) links[active].focus();
  });
  nav.addEventListener('click', event => {
    const index = links.indexOf(event.target.closest('a'));
    if (index < 0) return;
    setActive(index);
    close();
    sections[index].setAttribute('tabindex', '-1');
    sections[index].focus({ preventScroll:true });
  });
  const onKey = event => { if (event.key === 'Escape' && nav.dataset.open === 'true') { close(); toggle.focus(); } };
  const onOutside = event => { if (!nav.contains(event.target) && !toggle.contains(event.target)) close(); };
  nav.addEventListener('pointermove', onMove);
  nav.addEventListener('pointerleave', clearProximity);
  document.addEventListener('keydown', onKey);
  document.addEventListener('pointerdown', onOutside);
  window.addEventListener('scroll', onScroll, { passive:true });
  window.addEventListener('resize', onScroll, { passive:true });
  reduced.addEventListener('change', clearProximity);
  const onVisibility = () => { clearProximity(); if (document.hidden) { cancelAnimationFrame(scrollFrame); scrollFrame = 0; } else track(); };
  document.addEventListener('visibilitychange', onVisibility);
  const unsubscribe = paneMotion.subscribe(clearProximity);
  const themeObserver = new MutationObserver(clearProximity);
  themeObserver.observe(document.documentElement, { attributes:true, attributeFilter:['data-theme-transition'] });
  setActive(0); track();
  window.addEventListener('pagehide', event => {
    if (event.persisted) return;
    cancelAnimationFrame(raf); cancelAnimationFrame(scrollFrame); unsubscribe(); themeObserver.disconnect();
    document.removeEventListener('keydown', onKey);
    document.removeEventListener('pointerdown', onOutside);
    document.removeEventListener('visibilitychange', onVisibility);
    window.removeEventListener('scroll', onScroll); window.removeEventListener('resize', onScroll);
    reduced.removeEventListener('change', clearProximity);
    nav.remove(); toggle.remove();
  });
}
