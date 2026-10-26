/** Themed page transitions for Settings and the skin workshop.
 *
 * One overlay at a time; a second request interrupts the first (running its
 * pending `apply` once), so rapid clicks never stack stages. Reduced motion
 * applies immediately and leaves no overlay behind.
 */
import { gsap } from "gsap";
import {
  skinTransitionKind,
  transitionBudget,
  transitionOrigin,
  type SkinTransitionKind,
} from "./skinTransitionPlan";

export type TransitionSource = Element | { x: number; y: number } | null | undefined;

interface TransitionOptions {
  reduced: boolean;
  apply: () => void;
}

interface ActiveTransition {
  overlay: HTMLElement;
  timeline: gsap.core.Timeline;
  state: { applied: boolean };
  apply: () => void;
}

let active: ActiveTransition | null = null;

function sourcePoint(source: TransitionSource): { x: number; y: number } {
  if (source && typeof source === "object" && "x" in source && "y" in source) {
    return { x: source.x, y: source.y };
  }
  const rect =
    source && typeof Element !== "undefined" && source instanceof Element
      ? source.getBoundingClientRect()
      : null;
  return transitionOrigin(rect, { width: window.innerWidth, height: window.innerHeight });
}

function overlayFor(kind: SkinTransitionKind): HTMLElement {
  const overlay = document.createElement("div");
  overlay.className = `skin-transition skin-transition-${kind}`;
  overlay.dataset.engine = "gsap";
  overlay.setAttribute("aria-hidden", "true");
  overlay.style.cssText =
    "position:fixed;inset:0;z-index:2147483000;pointer-events:none;overflow:hidden;contain:strict;";
  return overlay;
}

function cleanup(overlay: HTMLElement): void {
  overlay.remove();
  if (active?.overlay === overlay) active = null;
}

function pixelTiles(overlay: HTMLElement, origin: { x: number; y: number }): HTMLElement[] {
  const size = 22;
  const width = window.innerWidth;
  const height = window.innerHeight;
  const cols = Math.ceil(width / size);
  const rows = Math.ceil(height / size);
  const tiles: HTMLElement[] = [];
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      const tile = document.createElement("i");
      tile.style.cssText = `position:absolute;left:${col * size}px;top:${row * size}px;width:${size}px;height:${size}px;background:var(--cp-yellow,#e9ff1f);border-right:1px solid rgb(8 10 13 / 18%);border-bottom:1px solid rgb(8 10 13 / 18%);will-change:transform,opacity;`;
      const dx = col * size + size / 2 - origin.x;
      const dy = row * size + size / 2 - origin.y;
      tile.dataset.distance = String(Math.round(Math.hypot(dx, dy)));
      tiles.push(tile);
    }
  }
  tiles.sort((a, b) => Number(a.dataset.distance) - Number(b.dataset.distance));
  overlay.append(...tiles);
  return tiles;
}

function playPixel(overlay: HTMLElement, origin: { x: number; y: number }, applyOnce: () => void): gsap.core.Timeline {
  const budget = transitionBudget("pixel");
  const tiles = pixelTiles(overlay, origin);
  const eachIn = Math.min(0.0016, budget.cover / 1000 / Math.max(1, tiles.length));
  const eachOut = Math.min(0.0012, budget.release / 1000 / Math.max(1, tiles.length));
  gsap.set(tiles, { scale: 0, opacity: 0, transformOrigin: "50% 50%" });
  const tl = gsap.timeline();
  tl.to(tiles, { scale: 1, opacity: 1, duration: 0.12, ease: "steps(2)", stagger: { each: eachIn } }, 0)
    .call(applyOnce, undefined, ">+0.02")
    .to(tiles, { scale: 0, opacity: 0, duration: 0.12, ease: "steps(2)", stagger: { each: eachOut } }, ">+0.02");
  return tl;
}

function playStamp(overlay: HTMLElement, origin: { x: number; y: number }, applyOnce: () => void): gsap.core.Timeline {
  const budget = transitionBudget("stamp");
  const radius = Math.hypot(
    Math.max(origin.x, window.innerWidth - origin.x),
    Math.max(origin.y, window.innerHeight - origin.y),
  );
  const cover = document.createElement("i");
  cover.style.cssText = "position:absolute;inset:0;background:var(--post-paper,#fffdf4);will-change:clip-path;";
  const ring = document.createElement("i");
  const ringSize = 168;
  ring.style.cssText = `position:absolute;left:${origin.x - ringSize / 2}px;top:${origin.y - ringSize / 2}px;width:${ringSize}px;height:${ringSize}px;border:5px double var(--post-ink,#3e5872);border-radius:50%;box-shadow:0 0 0 7px color-mix(in srgb,var(--post-ink,#3e5872) 18%,transparent);will-change:transform,opacity;`;
  overlay.append(cover, ring);
  gsap.set(cover, { clipPath: `circle(0px at ${origin.x}px ${origin.y}px)` });
  gsap.set(ring, { scale: 0.3, opacity: 0, rotation: -9, transformOrigin: "50% 50%" });
  const tl = gsap.timeline();
  tl.to(cover, { clipPath: `circle(${Math.ceil(radius)}px at ${origin.x}px ${origin.y}px)`, duration: budget.cover / 1000, ease: "power2.in" }, 0)
    .to(ring, { scale: 1, opacity: 1, rotation: 0, duration: 0.2, ease: "power4.out" }, 0.06)
    .to(ring, { rotation: -2.5, duration: 0.045 }, 0.27)
    .to(ring, { rotation: 1.5, duration: 0.045 }, 0.315)
    .to(ring, { rotation: 0, duration: 0.05 }, 0.36)
    .call(applyOnce, undefined, ">+0.01")
    .to([ring, cover], { opacity: 0, duration: budget.release / 1000, ease: "power2.out" }, ">+0.02");
  return tl;
}

function playInk(overlay: HTMLElement, origin: { x: number; y: number }, applyOnce: () => void): gsap.core.Timeline {
  const budget = transitionBudget("ink");
  const blobs: HTMLElement[] = [];
  const spread = Math.hypot(window.innerWidth, window.innerHeight) * 0.62;
  for (let i = 0; i < 6; i += 1) {
    const blob = document.createElement("i");
    const size = 150 + ((i * 37) % 90);
    const angle = (Math.PI * 2 * i) / 6 + 0.35;
    const offset = i === 0 ? 0 : spread * 0.34;
    blob.style.cssText = `position:absolute;left:${origin.x + Math.cos(angle) * offset - size / 2}px;top:${origin.y + Math.sin(angle) * offset - size / 2}px;width:${size}px;height:${size}px;background:var(--ink-stroke,#323532);border-radius:${38 + i * 4}% ${62 - i * 3}% ${55 + i * 2}% ${45 - i * 2}% / ${52 - i * 2}% ${42 + i * 3}% ${58 - i * 3}% ${48 + i * 2}%;filter:blur(${i === 0 ? 1 : 2.5}px);will-change:transform,opacity;`;
    overlay.append(blob);
    blobs.push(blob);
  }
  gsap.set(blobs, { scale: 0.06, opacity: 0.9, transformOrigin: "50% 50%" });
  const tl = gsap.timeline();
  tl.to(blobs, { scale: 9, opacity: 1, duration: budget.cover / 1000, ease: "power2.in", stagger: 0.025 }, 0)
    .call(applyOnce, undefined, ">+0.01")
    .to(blobs, { opacity: 0, duration: budget.release / 1000, ease: "power2.out", stagger: 0.018 }, ">+0.02");
  return tl;
}

/// Cancel any running transition. A pending `apply` still runs so an
/// interrupted click never becomes a no-op.
export function cancelSkinTransition(): void {
  const running = active;
  active = null;
  if (!running) return;
  running.timeline.kill();
  cleanup(running.overlay);
  if (!running.state.applied) {
    running.state.applied = true;
    running.apply();
  }
}

/// Play the theme's transition, then run `apply` exactly once while the
/// overlay covers the page. No themed pack (native, unknown) applies at once.
export function playSkinTransition(
  skin: string | null | undefined,
  source: TransitionSource,
  options: TransitionOptions,
): void {
  const kind = skinTransitionKind(skin);
  if (!kind || options.reduced) {
    cancelSkinTransition();
    options.apply();
    return;
  }
  cancelSkinTransition();
  const overlay = overlayFor(kind);
  const origin = sourcePoint(source);
  document.body.append(overlay);
  const state = { applied: false };
  const applyOnce = () => {
    if (state.applied) return;
    state.applied = true;
    options.apply();
  };
  const timeline =
    kind === "pixel"
      ? playPixel(overlay, origin, applyOnce)
      : kind === "stamp"
        ? playStamp(overlay, origin, applyOnce)
        : playInk(overlay, origin, applyOnce);
  active = { overlay, timeline, state, apply: options.apply };
  timeline.eventCallback("onComplete", () => {
    cleanup(overlay);
  });
  timeline.play(0);
}
