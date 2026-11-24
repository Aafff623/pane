import { DEMO_GUIDE, type DemoGuide } from "../src/content/demo-guide";

const anchors: Partial<Record<DemoGuide, string>> = {
  overview: ".quota-overview .provider-head",
  header: ".total-spend .provider-head",
  actions: ".quota-overview .overview-switch-row",
  accounts: 'article[data-provider="clinepass"] .provider-head',
  spend: ".total-spend .donut-wrap",
  settings: "#settings-btn",
};

let selected: DemoGuide | undefined;
let marked: HTMLElement | null = null;
let revealPending = false;

function markGuide() {
  const selector = selected && anchors[selected];
  const target = selector ? document.querySelector<HTMLElement>(selector) : null;
  if (target !== marked) {
    marked?.removeAttribute("data-pane-guide-focus");
    target?.setAttribute("data-pane-guide-focus", "");
    marked = target;
  }

  const scroller = document.querySelector<HTMLElement>("#providers");
  if (!target || !revealPending || !scroller) return;
  revealPending = false;
  if (!scroller.contains(target)) return;

  const area = scroller.getBoundingClientRect();
  const item = target.getBoundingClientRect();
  if (item.top >= area.top + 10 && item.bottom <= area.bottom - 10) return;

  const scale = area.height / scroller.clientHeight;
  if (!scale) return;
  const centerOffset = Math.max(16, (area.height - item.height) / 2);
  scroller.scrollTo({
    top: scroller.scrollTop + (item.top - area.top - centerOffset) / scale,
    behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth",
  });
}

export function installDemoGuide() {
  const providers = document.querySelector("#providers");
  if (!providers) return;
  const observer = new MutationObserver(markGuide);
  // Re-render replaces nodes (childList); folding a card only flips a class,
  // and the guided target can live inside a section that just reopened.
  observer.observe(providers, { subtree: true, childList: true, attributes: true, attributeFilter: ["class"] });

  window.addEventListener("message", (event) => {
    if (event.source !== parent || event.origin !== location.origin || event.data?.type !== "pane-demo-guide") return;
    const step = DEMO_GUIDE.find((item) => item === event.data.step);
    if (!step) return;
    selected = step;
    revealPending = true;
    document.body.toggleAttribute("data-pane-guide-settings", selected === "settings");
    const scroller = document.querySelector<HTMLElement>("#providers");
    scroller?.scrollTo({ top: scroller.scrollTop, behavior: "instant" });

    if (selected === "actions") {
      document.querySelector<HTMLElement>(".quota-overview.is-folded [data-overview-fold]")?.click();
    }
    if (selected === "spend") {
      document.querySelector<HTMLElement>(".total-spend.is-folded [data-spend-fold]")?.click();
    }
    requestAnimationFrame(markGuide);
  });
}
