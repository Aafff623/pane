// Small primitives shared by the site's interactive pieces. Each one exists
// because the HeroUI / React Aria component of the same name ships it and the
// site's hand-rolled version was missing it: Intl-backed number formatting,
// the global hover-open coordination between preview cards, roving focus for a
// group of controls, and long-press for touch (which has no hover at all).

// Intl.NumberFormat construction is the expensive part; the site formats the
// same handful of shapes many times, so cache by options key.
const formatters = new Map();
function formatter(options) {
  const key = JSON.stringify(options);
  let f = formatters.get(key);
  if (!f) { f = new Intl.NumberFormat('en-US', options); formatters.set(key, f); }
  return f;
}

// en-US on purpose: the panel itself renders "1.2M tokens" and "$18.40", so the
// site's sample numbers have to read the same in both languages.
export const formatCurrency = (value, currency = 'USD', maximumFractionDigits = 2) =>
  formatter({ style: 'currency', currency, maximumFractionDigits }).format(value);
export const formatPercent = (value, maximumFractionDigits = 0) =>
  formatter({ style: 'percent', maximumFractionDigits }).format(value);
export const formatCompact = (value) => formatter({ notation: 'compact', maximumFractionDigits: 1 }).format(value);
export const formatCount = (value) => formatter({ maximumFractionDigits: 0 }).format(value);

export function formatByKind(kind, value) {
  if (kind === 'currency') return formatCurrency(value);
  if (kind === 'percent') return formatPercent(value);
  if (kind === 'compact') return formatCompact(value);
  return formatCount(value);
}

// One warm window shared by every preview card: once any card has opened, the
// next one opens immediately instead of waiting out its own delay again.
// React Aria coordinates hover cards and tooltips exactly this way.
let warmUntil = 0;
export const warmHover = (ms = 600) => { warmUntil = performance.now() + ms; };
export const isHoverWarm = () => performance.now() < warmUntil;

// Arrow-key movement inside a group of controls, wrapping at the ends.
// Returns false when the key is not the group's business, so the caller can
// fall through to its own handling.
export function moveRovingFocus(items, current, event, { orientation = 'horizontal' } = {}) {
  const forward = orientation === 'vertical' ? 'ArrowDown' : 'ArrowRight';
  const backward = orientation === 'vertical' ? 'ArrowUp' : 'ArrowLeft';
  const step = event.key === forward ? 1 : event.key === backward ? -1 : 0;
  if (!step) return false;
  const enabled = items.filter((el) => !el.disabled);
  const from = enabled.indexOf(current);
  if (from < 0) return false;
  event.preventDefault();
  enabled[(from + step + enabled.length) % enabled.length].focus();
  return true;
}

// One switch, several buttons: every pause control reads and writes the shared
// state and shows the same label, so "paused" cannot mean two different things
// depending on which section the visitor happens to be looking at.
export function bindMotionToggle(button, paneMotion, label) {
  if (!button) return;
  paneMotion.subscribe((paused) => {
    button.setAttribute('aria-pressed', String(paused));
    button.textContent = label(paused);
  });
  button.addEventListener('click', () => paneMotion.toggle());
}

// Touch has no hover, so a long press opens the preview — React Aria's
// PreviewTrigger does the same. Pointer events keep one path for mouse+touch.
export function onLongPress(el, handler, delay = 500) {
  let timer = 0;
  const cancel = () => { clearTimeout(timer); timer = 0; };
  el.addEventListener('pointerdown', (event) => {
    if (event.pointerType !== 'touch') return;
    cancel();
    timer = setTimeout(() => handler(event), delay);
  });
  for (const type of ['pointerup', 'pointercancel', 'pointerleave', 'scroll']) {
    el.addEventListener(type, cancel, { passive: true });
  }
  return cancel;
}
