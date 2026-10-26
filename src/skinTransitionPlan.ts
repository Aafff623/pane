/** Pure transition planning, kept separate from GSAP so it is testable. */
export type SkinTransitionKind = "pixel" | "stamp" | "ink";

/// Only the three official packs get a themed transition; native stays
/// instant. Any future pack without a designed timeline must not inherit
/// another skin's animation.
export function skinTransitionKind(skin: string | null | undefined): SkinTransitionKind | null {
  switch (skin) {
    case "comic-pixel":
      return "pixel";
    case "signal":
      return "stamp";
    case "ink":
      return "ink";
    default:
      return null;
  }
}

/// One source point for every timeline. A missing/zero rect (hidden page)
/// falls back to the viewport centre instead of reading a hidden width.
export function transitionOrigin(
  rect: { left: number; top: number; width: number; height: number } | null | undefined,
  viewport: { width: number; height: number },
): { x: number; y: number } {
  if (!rect || rect.width <= 0 || rect.height <= 0) {
    return { x: viewport.width / 2, y: viewport.height / 2 };
  }
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

/// 350–650ms per timeline; exposed so the DOM build can budget its stagger.
export function transitionBudget(kind: SkinTransitionKind): { cover: number; hold: number; release: number } {
  switch (kind) {
    case "pixel":
      return { cover: 260, hold: 60, release: 220 };
    case "stamp":
      return { cover: 280, hold: 90, release: 220 };
    case "ink":
      return { cover: 300, hold: 80, release: 230 };
  }
}
