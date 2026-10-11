// One pause state for every ambient animation on the page.
//
// The why-pane collector, the ecosystem orbit, the background grid and the
// feature fragments used to own four independent switches — two of them
// visible buttons with different labels ("暂停动画" / "暂停轨道"), two of them
// invisible. "Pause" therefore meant a different thing in each place, and
// pausing the orbit left the collector flying. They now share one state:
// persisted in localStorage, seeded from the visitor's reduced-motion
// preference, and broadcast to every subscriber.
const KEY = 'pane-site-motion';
const reduce = matchMedia('(prefers-reduced-motion: reduce)');

let stored = null;
try { stored = localStorage.getItem(KEY); } catch {}
let paused = stored === null ? reduce.matches : stored === 'paused';

const listeners = new Set();

export const paneMotion = {
  get paused() { return paused; },
  set(value) {
    const next = !!value;
    if (next === paused) return;
    paused = next;
    try { localStorage.setItem(KEY, paused ? 'paused' : 'running'); } catch {}
    for (const fn of listeners) fn(paused);
  },
  toggle() { this.set(!paused); },
  // Runs fn immediately with the current value, then on every change.
  subscribe(fn) { listeners.add(fn); fn(paused); return () => listeners.delete(fn); },
};
