// Scroll-scrub for the three narrative boundaries (#87): why-pane → demo →
// ecosystem. Each section carries a `--scrub` number that dips toward 0 as it
// crosses into or out of the viewport and rests at 1 while it holds the screen,
// so the copy hands over to the next section instead of both being fully lit.
//
// Two rules shape the implementation:
//   * No per-frame layout reads. Document offsets are measured once and cached;
//     a scroll frame only reads `scrollY` and writes a custom property. Asking
//     the browser for a section's box on every frame is what turns a scrub into
//     jank, because the read forces the previous frame's write to lay out.
//   * The shared switch is authoritative. Pausing motion settles every section
//     at 1 and stops the loop rather than freezing it mid-dip, and reduced
//     motion never starts it at all.

import { paneMotion } from '/motion.js?v=2';
import { subscribe as subscribeTicker } from '/ticker.js?v=1';
import { scrubValue } from './scrub-curve.js?v=1';

const EPSILON = 0.002;

// `offsetTop` is relative to the offset parent, so walk the chain to get a
// document coordinate that survives the sections being nested in wrappers.
function documentOffset(node) {
  let top = 0;
  for (let current = node; current; current = current.offsetParent) top += current.offsetTop;
  return top;
}

export function initScrollScrub() {
  const sections = [...globalThis.document?.querySelectorAll('[data-scrub]') ?? []];
  if (!sections.length) return null;
  const reduce = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)');
  const entries = sections.map((section) => ({ section, top: 0, height: 0, near: false, last: -1 }));

  let unsubscribe = null;
  let dirty = false;
  let observer = null;

  function settle() {
    for (const entry of entries) {
      entry.last = 1;
      entry.section.style.setProperty('--scrub', '1');
    }
  }

  function paint() {
    const y = globalThis.scrollY ?? 0;
    const viewport = globalThis.innerHeight ?? 0;
    for (const entry of entries) {
      if (!entry.near) continue;
      const next = scrubValue(y, viewport, entry.top, entry.height);
      if (Math.abs(next - entry.last) < EPSILON) continue;
      entry.last = next;
      entry.section.style.setProperty('--scrub', next.toFixed(3));
    }
  }

  function frame() {
    if (!dirty) { sleep(); return; }
    dirty = false;
    paint();
  }

  function wake() {
    if (paneMotion.paused || reduce?.matches) return;
    dirty = true;
    if (!unsubscribe) unsubscribe = subscribeTicker(frame);
  }

  function sleep() {
    const off = unsubscribe;
    unsubscribe = null;
    off?.();
  }

  // Re-measure whenever anything can move the sections: the viewport, the web
  // fonts, the demo iframe booting, or a `<details>` inside a scrubbed section
  // opening. Measuring is the only place a layout read is allowed.
  function measure() {
    for (const entry of entries) {
      entry.top = documentOffset(entry.section);
      entry.height = entry.section.offsetHeight;
      entry.last = -1;
    }
    if (paneMotion.paused || reduce?.matches) settle();
    else paint();
  }

  function refresh() {
    measure();
    if (entries.some((entry) => entry.near)) wake();
  }

  if (globalThis.IntersectionObserver) {
    observer = new IntersectionObserver((records) => {
      for (const record of records) {
        const entry = entries.find((candidate) => candidate.section === record.target);
        if (!entry || entry.near === record.isIntersecting) continue;
        entry.near = record.isIntersecting;
        // A section arriving on screen may not have been measured since the
        // last layout change, so re-measure it before the first painted frame.
        if (entry.near) {
          entry.top = documentOffset(entry.section);
          entry.height = entry.section.offsetHeight;
          entry.last = -1;
        }
      }
      if (entries.some((candidate) => candidate.near)) wake();
      else sleep();
    }, { rootMargin: '30% 0px' });
    for (const entry of entries) observer.observe(entry.section);
  } else {
    for (const entry of entries) entry.near = true;
  }

  globalThis.addEventListener?.('scroll', wake, { passive: true });
  globalThis.addEventListener?.('resize', refresh);
  globalThis.addEventListener?.('pagehide', () => { sleep(); observer?.disconnect(); }, { once: true });
  globalThis.document?.addEventListener('visibilitychange', () => (globalThis.document.hidden ? sleep() : refresh()));
  reduce?.addEventListener?.('change', refresh);
  paneMotion.subscribe(refresh);
  globalThis.document?.fonts?.ready?.then(refresh);
  for (const section of sections) {
    for (const details of section.querySelectorAll('details')) details.addEventListener('toggle', refresh);
  }

  return { measure, settle, paint, entries };
}
