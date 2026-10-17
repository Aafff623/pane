// MOTDES PixelSwap grid/order and pixel growth, adapted to a single incoming
// View Transition snapshot instead of cloning the entire site for each pixel.
const MAX_PIXELS = 220;
const STEPS = 48;
const clamp = value => Math.min(1, Math.max(0, value));
const noise = seed => {
  const value = Math.sin(seed * 127.1 + 311.7) * 43758.5453;
  return value - Math.floor(value);
};
export const pixelSwapSettings = Object.freeze({ pixelSize:64, pixelScale:0.35, duration:850, pixelDuration:300 });

export function buildPixelGrid(width, height, requestedSize = 64) {
  let size = Math.max(8, Math.round(requestedSize));
  let columns = Math.max(1, Math.ceil(width / size));
  let rows = Math.max(1, Math.ceil(height / size));
  // The supplied grid does one enlargement. Repeat it to keep the actual cap
  // true even when rounding adds a row/column on a large or very narrow screen.
  while (columns * rows > MAX_PIXELS) {
    size = Math.max(size + 1, Math.ceil(size * Math.sqrt(columns * rows / MAX_PIXELS)));
    columns = Math.max(1, Math.ceil(width / size));
    rows = Math.max(1, Math.ceil(height / size));
  }
  const originX = (width - columns * size) / 2;
  const originY = (height - rows * size) / 2;
  return { size, pixels:Array.from({ length:columns * rows }, (_, index) => ({
    left:originX + index % columns * size,
    top:originY + Math.floor(index / columns) * size,
    offset:noise(index + 1),
  })) };
}

// The supplied cubic-bezier(0.22, 1, 0.36, 1), sampled once for each grid frame.
function ease(progress) {
  let t = progress;
  for (let i = 0; i < 5; i++) {
    const slope = (3 * 0.58 * t - 0.48) * t + 0.66;
    if (!slope) break;
    t -= (((0.58 * t - 0.24) * t + 0.66) * t - progress) / slope;
  }
  t = clamp(t);
  return 1 - (1 - t) ** 3;
}

export function buildPixelClipFrames(grid, settings = pixelSwapSettings) {
  const spread = settings.duration - settings.pixelDuration;
  const number = value => value.toFixed(2);
  return Array.from({ length:STEPS + 1 }, (_, step) => {
    const offset = step / STEPS, elapsed = offset * settings.duration;
    const path = grid.pixels.map(pixel => {
      const progress = clamp((elapsed - pixel.offset * spread) / settings.pixelDuration);
      // Opening geometry also supplies the fade: the first tiny window grows
      // from zero, so delayed pixels never pop into the old page at time zero.
      const scale = (settings.pixelScale + (1 - settings.pixelScale) * ease(progress)) * Math.min(1, progress * 5);
      const edge = (grid.size + 1) * scale;
      const x = pixel.left + (grid.size - edge) / 2;
      const y = pixel.top + (grid.size - edge) / 2;
      return `M${number(x)} ${number(y)}h${number(edge)}v${number(edge)}h${number(-edge)}Z`;
    }).join('');
    return { offset, clipPath:`path("${path}")` };
  });
}

export function initThemeSwap({ root, button, syncEffects = () => {}, motionPaused = () => false, doc = document, win = window }) {
  let desired = root.dataset.theme === 'light' ? 'light' : 'dark';
  let running = false, transition, animation, disposed = false;
  const reduce = win.matchMedia('(prefers-reduced-motion: reduce)');
  const syncButton = () => {
    button.querySelector('img').src = `/icons/${root.dataset.theme === 'light' ? 'moon' : 'sun'}.svg`;
    button.setAttribute('aria-pressed', String(root.dataset.theme === 'light'));
    doc.querySelector('meta[name="theme-color"]')?.setAttribute('content', root.dataset.theme === 'light' ? '#f4f5f3' : '#08090a');
  };
  const commit = async theme => {
    root.dataset.theme = theme;
    try { win.localStorage.setItem('pane-site-theme', theme); } catch {}
    syncButton();
    await syncEffects();
  };
  const clear = () => {
    animation?.cancel(); animation = null;
    delete root.dataset.themeTransition;
    button.removeAttribute('aria-busy');
  };
  const swap = async theme => {
    if (!doc.startViewTransition || !root.animate || reduce.matches || motionPaused() || doc.hidden) {
      await commit(theme);
      return;
    }
    // PixelSwap owns the transition; suppress the native crossfade, local CSS
    // colour transitions and cursor fluid while its screenshots are displayed.
    root.dataset.themeTransition = 'pixel';
    button.setAttribute('aria-busy', 'true');
    const grid = buildPixelGrid(win.innerWidth, win.innerHeight, pixelSwapSettings.pixelSize);
    const frames = buildPixelClipFrames(grid);
    try {
      transition = doc.startViewTransition(() => commit(theme));
      // finished and ready reject separately if the update callback fails.
      // Observe both immediately, including the path where ready rejects first.
      transition.finished.catch(() => {});
      await transition.ready;
      if (disposed) { transition.skipTransition(); return; }
      animation = root.animate(frames, {
        duration:pixelSwapSettings.duration, easing:'linear', fill:'both',
        pseudoElement:'::view-transition-new(root)',
      });
      await animation.finished.catch(() => {});
      await transition.finished;
    } catch {
      transition?.skipTransition();
      // ready can reject when the document hides or another transition starts.
      // The update callback still owns the theme commit in that case.
      if (transition) {
        await transition.updateCallbackDone.catch(() => {});
        await transition.finished.catch(() => {});
      }
      else await commit(theme);
    } finally { transition = null; clear(); }
  };
  const onClick = async () => {
    desired = desired === 'light' ? 'dark' : 'light';
    if (running) { transition?.skipTransition(); animation?.cancel(); return; }
    running = true;
    try {
      while (!disposed && root.dataset.theme !== desired) await swap(desired);
    } finally { running = false; clear(); }
  };
  const onResize = () => { transition?.skipTransition(); animation?.cancel(); };
  // A captured root is hit-tested as HTML while the snapshot is on screen.
  // Preserve a second click on the visible theme button during that interval.
  const onSnapshotClick = event => {
    if (!running || event.target !== root) return;
    const box = button.getBoundingClientRect();
    if (event.clientX >= box.left && event.clientX <= box.right && event.clientY >= box.top && event.clientY <= box.bottom) onClick();
  };
  const onVisibility = () => { if (doc.hidden) onResize(); };
  const onReduce = () => { if (reduce.matches) onResize(); };
  syncButton();
  button.addEventListener('click', onClick);
  win.addEventListener('resize', onResize, { passive:true });
  doc.addEventListener('visibilitychange', onVisibility);
  doc.addEventListener('click', onSnapshotClick);
  reduce.addEventListener('change', onReduce);
  return { destroy() {
    disposed = true; onResize(); clear();
    button.removeEventListener('click', onClick);
    win.removeEventListener('resize', onResize);
    doc.removeEventListener('visibilitychange', onVisibility);
    doc.removeEventListener('click', onSnapshotClick);
    reduce.removeEventListener('change', onReduce);
  } };
}
