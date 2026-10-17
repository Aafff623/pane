// Native adapters for the owner-supplied MOTDES components. No React runtime.
import { paneMotion } from '/motion.js?v=1';

export function initSiteEffects() {
  const finePointer = matchMedia('(hover: hover) and (pointer: fine)');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const cursorToggle = document.querySelector('#demo-cursor-toggle');
  let cursorEnabled = true;
  try { cursorEnabled = localStorage.getItem('pane-demo-target-cursor') !== 'off'; } catch {}
  const syncCursorToggle = () => {
    if (!cursorToggle) return;
    const english = document.documentElement.lang === 'en';
    cursorToggle.setAttribute('aria-pressed', String(cursorEnabled));
    cursorToggle.querySelector('span').textContent = english ? 'Target cursor' : '框选跟随';
    cursorToggle.title = english
      ? (cursorEnabled ? 'Turn off target cursor' : 'Turn on target cursor')
      : (cursorEnabled ? '关闭框选跟随效果' : '开启框选跟随效果');
  };
  const glowLayer = document.createElement('div');
  glowLayer.className = 'site-glow';
  glowLayer.setAttribute('aria-hidden', 'true');
  const canvas = document.createElement('canvas');
  glowLayer.append(canvas);
  document.body.append(glowLayer);

  let glow, glowPending = false, disposed = false;
  const enabled = () => finePointer.matches && !reduced.matches && !paneMotion.paused;
  const light = () => document.documentElement.dataset.theme === 'light';
  const syncDemoCursor = () => {
    const paused = !cursorEnabled || !enabled() || document.hidden || document.documentElement.hasAttribute('data-theme-transition');
    document.querySelectorAll('iframe').forEach(frame => {
      if (frame.getAttribute('src')?.startsWith('/demo/')) {
        frame.contentWindow?.postMessage({ type:'pane-demo-motion', paused }, location.origin);
      }
    });
  };
  const sync = () => {
    syncCursorToggle();
    syncDemoCursor();
    const active = enabled();
    const interactive = active && !document.documentElement.hasAttribute('data-theme-transition');
    glowLayer.hidden = !interactive;
    glow?.update({ enabled:interactive, theme:light() ? 'light' : 'dark' });
  };
  const toggleCursor = () => {
    cursorEnabled = !cursorEnabled;
    try { localStorage.setItem('pane-demo-target-cursor', cursorEnabled ? 'on' : 'off'); } catch {}
    syncCursorToggle();
    syncDemoCursor();
  };
  cursorToggle?.addEventListener('click', toggleCursor);
  syncCursorToggle();
  async function loadGlow(event) {
    if (disposed || glow || glowPending || event.pointerType !== 'mouse' || !enabled() || document.documentElement.hasAttribute('data-theme-transition')) return;
    glowPending = true;
    try {
      const { createGlowCursor } = await import('/effects/glow-cursor.js?v=1');
      if (!disposed && enabled() && !document.documentElement.hasAttribute('data-theme-transition')) {
        glow = createGlowCursor(glowLayer, canvas, { theme:light() ? 'light' : 'dark' });
        glow?.update({ theme:light() ? 'light' : 'dark' });
        // A device without the required WebGL extensions should not retry on every move.
        window.removeEventListener('pointermove', loadGlow);
      }
    } catch { glowLayer.hidden = true; window.removeEventListener('pointermove', loadGlow); }
    finally { glowPending = false; }
  }
  const themeObserver = new MutationObserver(sync);
  themeObserver.observe(document.documentElement, { attributes:true, attributeFilter:['data-theme','data-theme-transition'] });
  const unsubscribe = paneMotion.subscribe(sync);
  document.addEventListener('load', syncDemoCursor, true);
  document.addEventListener('visibilitychange', syncDemoCursor);
  finePointer.addEventListener('change', sync);
  reduced.addEventListener('change', sync);
  window.addEventListener('pointermove', loadGlow, { passive:true });
  window.addEventListener('pagehide', event => {
    if (event.persisted) return;
    disposed = true;
    cursorToggle?.removeEventListener('click', toggleCursor);
    themeObserver.disconnect(); unsubscribe();
    document.removeEventListener('load', syncDemoCursor, true);
    document.removeEventListener('visibilitychange', syncDemoCursor);
    finePointer.removeEventListener('change', sync);
    reduced.removeEventListener('change', sync);
    window.removeEventListener('pointermove', loadGlow);
    glow?.destroy();
    glowLayer.remove();
  });
  return { syncTheme:sync };
}
