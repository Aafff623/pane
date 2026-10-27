// The scroll-scrub curve (#87), kept free of DOM and module imports so the site
// tests can pin its boundaries without a browser. scroll-scrub.js owns the
// wiring; this file owns the shape.

// How much of the viewport each boundary takes to settle. Half a screen reads
// as a hand-over; a full screen would still be fading once the next section is
// already the one being read.
export const SPAN = 0.5;

const clamp01 = (value) => (value < 0 ? 0 : value > 1 ? 1 : value);
const smooth = (t) => t * t * (3 - 2 * t);

/// Returns 0 at the moment the section's top edge meets the bottom of the
/// viewport, 1 once it is a whole `SPAN` of viewport past it, and back down to
/// 0 as its bottom edge leaves the top. Smoothstepped so the slope is zero at
/// both ends — a raw clamp would show a visible corner where it saturates.
export function scrubValue(scrollY, viewport, top, height) {
  const span = Math.max(1, viewport * SPAN);
  const entering = clamp01((scrollY + viewport - top) / span);
  const leaving = clamp01((top + height - scrollY) / span);
  return smooth(Math.min(entering, leaving));
}
