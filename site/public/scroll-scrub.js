// Scroll-scrub for the three narrative boundaries (#87): why-pane → demo →
// ecosystem. Each section's copy blocks carry a `--scrub` number that dips
// toward 0 as the section crosses into or out of the viewport and rests at 1
// while it holds the screen, so the copy hands over to the next section instead
// of both being fully lit at once.
//
// Three rules shape the implementation:
//   * No per-frame layout reads. Document offsets are cached and re-measured
//     only when something can actually move them — resize, late web fonts, a
//     `<details>` opening, or an observed box changing size. A scroll frame
//     reads `scrollY` and writes custom properties. Asking the browser for a
//     box every frame is what turns a scrub into jank, because the read forces
//     the previous frame's write to lay out.
//   * Write on the element that uses the variable, not on the section. An
//     inherited custom property dirties the whole subtree, and `#demo` contains
//     the embedded panel — publishing there would recalculate style for all of
//     it on every frame of a boundary crossing.
//   * The shared switch is authoritative. Pausing settles every block at 1 and
//     stops the loop rather than freezing it mid-dip, and reduced motion never
//     starts it at all.
//
// There is deliberately no IntersectionObserver gate here. An earlier draft only
// painted sections the observer had marked "near", and on a loaded machine the
// observer lagged far enough behind the scroll position that a section already
// on screen still carried its CSS fallback (fully opaque) and then dimmed in one
// jump. Painting every section costs less than that bug: a section away from a
// boundary is saturated at 0 or 1, so the epsilon check writes nothing.

import { paneMotion } from '/motion.js?v=2';
import { subscribe as subscribeTicker } from '/ticker.js?v=1';
import { scrubValue } from './scrub-curve.js?v=1';

// Below this change nothing is written. Three decimals is already finer than an
// 8-bit opacity channel can express.
const EPSILON = 0.002;

// `offsetTop` is relative to the offset parent, so walk the chain to get a
// document coordinate that survives the sections being nested in wrappers.
function documentOffset(node) {
  let top = 0;
  for (let current = node; current; current = current.offsetParent) top += current.offsetTop;
  return top;
}

export function initScrollScrub() {
  const document = globalThis.document;
  const sections = [...(document?.querySelectorAll('[data-scrub]') ?? [])];
  const entries = sections
    .map((section) => ({
      section,
      targets: [...section.querySelectorAll('[data-scrub-fade]')],
      top: 0,
      height: 0,
      last: Number.NaN,
    }))
    .filter((entry) => entry.targets.length);
  if (!entries.length) return null;

  const reduce = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)');
  const still = () => paneMotion.paused || Boolean(reduce?.matches);

  let unsubscribe = null;
  let dirty = false;
  let resizer = null;

  function write(entry, value) {
    const text = value.toFixed(3);
    for (const target of entry.targets) target.style.setProperty('--scrub', text);
  }

  /// Park every block at its settled state. Used for reduced motion and for the
  /// shared pause switch, where freezing mid-dip would leave the page dimmed.
  function settle() {
    for (const entry of entries) {
      entry.last = 1;
      write(entry, 1);
    }
  }

  function paint() {
    const y = globalThis.scrollY ?? 0;
    const viewport = globalThis.innerHeight ?? 0;
    for (const entry of entries) {
      const next = scrubValue(y, viewport, entry.top, entry.height);
      if (Math.abs(next - entry.last) < EPSILON) continue;
      entry.last = next;
      write(entry, next);
    }
  }

  function frame() {
    // One idle frame after the last scroll event the loop drops itself, so a
    // visitor who stops reading costs nothing until they move again.
    if (!dirty) { sleep(); return; }
    dirty = false;
    paint();
  }

  function wake() {
    if (still()) return;
    dirty = true;
    if (!unsubscribe) unsubscribe = subscribeTicker(frame);
  }

  function sleep() {
    const off = unsubscribe;
    unsubscribe = null;
    off?.();
  }

  // Measuring is the only place a layout read is allowed.
  function measure() {
    for (const entry of entries) {
      entry.top = documentOffset(entry.section);
      entry.height = entry.section.offsetHeight;
      entry.last = Number.NaN;
    }
    if (still()) settle();
    else paint();
  }

  function refresh() {
    measure();
    wake();
  }

  // The page keeps growing after init: the demo iframe loads lazily and roughly
  // doubles `#demo`'s height when it arrives, web fonts land late, and the
  // ecosystem roadmap is a `<details>` the visitor can open. Any of those
  // silently invalidates a cached offset, so watch the boxes rather than guess
  // which events matter. ResizeObserver already delivers at most once per frame.
  if (globalThis.ResizeObserver) {
    resizer = new ResizeObserver(() => { if (!still()) measure(); });
    for (const entry of entries) resizer.observe(entry.section);
    if (document?.body) resizer.observe(document.body);
  }

  globalThis.addEventListener?.('scroll', wake, { passive: true });
  globalThis.addEventListener?.('resize', refresh);
  globalThis.addEventListener?.('load', measure);
  globalThis.addEventListener?.('pagehide', () => { sleep(); resizer?.disconnect(); }, { once: true });
  document?.addEventListener('visibilitychange', () => (document.hidden ? sleep() : refresh()));
  reduce?.addEventListener?.('change', refresh);
  paneMotion.subscribe(refresh);
  document?.fonts?.ready?.then(measure);
  for (const entry of entries) {
    for (const details of entry.section.querySelectorAll('details')) details.addEventListener('toggle', measure);
  }

  measure();
  return { measure, settle, paint, entries };
}
