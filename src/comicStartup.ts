import { gsap } from "gsap";
import type { SkinId } from "./skins";
import "./comicStartup.css";

/// Pack skins that own a startup sequence of their own. One list, so
/// `themeStartup` and the workshop never disagree about who has one.
const COMIC_STARTUP_SKINS = new Set<SkinId>(["comic-pixel"]);

export function hasComicStartup(skin: SkinId): boolean {
  return COMIC_STARTUP_SKINS.has(skin);
}

export interface ComicStartupLabels {
  name: string;
  tagline: string;
  skip: string;
  /** Localized overlay description for screen readers. */
  aria: string;
}

let stopIntro: (() => void) | undefined;

function element<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/// Three panels → wordmark → seal, bounded to 3.3 s; click, Escape or the
/// skip button ends it. Animation failures degrade to the static frame
/// instead of breaking startup, and the returned cleanup is safe to repeat.
export function playComicStartup(reduced: boolean, labels: ComicStartupLabels): () => void {
  stopIntro?.();
  const previousFocus = document.activeElement;
  const overlay = element("section", "cm-intro");
  overlay.setAttribute("role", "dialog");
  overlay.setAttribute("aria-modal", "true");
  overlay.setAttribute("aria-label", labels.aria);
  overlay.dataset.phase = reduced ? "settled" : "playing";
  const art = element("div", "cm-intro-art");
  art.setAttribute("aria-hidden", "true");
  const panels = ["one", "two", "three"].map((name) => {
    const panel = element("div", `cm-intro-panel cm-intro-panel-${name}`);
    art.append(panel);
    return panel;
  });
  panels[0].append(element("i", "cm-intro-rays"));
  panels[1].append(element("i", "cm-intro-dots"));
  const wordmark = element("strong", "cm-intro-wordmark", "PANE");
  panels[2].append(wordmark);
  const seal = element("span", "cm-intro-seal", "P");
  art.append(seal);
  const caption = element("div", "cm-intro-caption");
  caption.append(element("h1", "cm-intro-name", labels.name), element("p", "cm-intro-tagline", labels.tagline));
  const skip = element("button", "cm-intro-skip", labels.skip);
  skip.type = "button";
  overlay.append(art, caption, skip);

  let closed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let timeline: gsap.core.Timeline | undefined;
  const close = () => {
    if (closed) return;
    closed = true;
    clearTimeout(timer);
    timeline?.kill();
    overlay.removeEventListener("click", close);
    document.removeEventListener("keydown", onKey, true);
    overlay.remove();
    if (stopIntro === close) {
      stopIntro = undefined;
      if (document.documentElement.dataset.themeIntro === "comic-pixel") {
        document.documentElement.removeAttribute("data-theme-intro");
      }
    }
    if (previousFocus instanceof HTMLElement && previousFocus.isConnected) {
      previousFocus.focus({ preventScroll: true });
    }
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.isComposing) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopImmediatePropagation();
      close();
    } else if (event.key === "Tab") {
      event.preventDefault();
      skip.focus({ preventScroll: true });
    }
  };
  stopIntro = close;
  document.documentElement.dataset.themeIntro = "comic-pixel";
  document.body.append(overlay);
  overlay.addEventListener("click", close);
  document.addEventListener("keydown", onKey, true);
  skip.focus({ preventScroll: true });

  if (!reduced) {
    try {
      timeline = gsap.timeline({ defaults: { ease: "power3.out" }, onComplete: close });
      timeline
        .fromTo(panels, { scaleX: 0, opacity: 0, transformOrigin: "0% 50%" }, { scaleX: 1, opacity: 1, duration: 0.32, stagger: 0.2 }, 0)
        .fromTo(wordmark, { opacity: 0, y: 14 }, { opacity: 1, y: 0, duration: 0.36 }, 0.8)
        .fromTo(caption, { opacity: 0, y: 7 }, { opacity: 1, y: 0, duration: 0.4 }, 1.15)
        .fromTo(seal, { scale: 1.65, opacity: 0, rotation: -21 }, { scale: 1, opacity: 1, rotation: -9, duration: 0.2, ease: "power3.in" }, 1.65)
        .to(overlay, { opacity: 0, duration: 0.45, ease: "power2.inOut" }, 2.85);
    } catch {
      // A dead ticker must not leave the dialog up: show the settled frame
      // and let the lifetime timer below close it.
      timeline?.kill();
      timeline = undefined;
      overlay.dataset.phase = "settled";
    }
  }
  // Also bounds lifetime when the animation ticker is suspended by the host.
  timer = setTimeout(close, reduced ? 1100 : 3300);
  return close;
}
