// One reveal gate for the page (#87). Two separate systems used to decide when
// an element plays its entrance: `.reveal-text` built a fresh IntersectionObserver
// per heading, and the section reveals shared a second observer with a different
// threshold. Neither knew about the other, so the two thresholds had drifted and
// a heading could still be blurred after the section around it had appeared.
//
// This module is the single registry. Observers are shared per (threshold,
// rootMargin) pair rather than per element, and each node declares the class its
// own CSS waits for, so the two vocabularies (`is-revealed`, `is-visible`) keep
// working without two gates. "Seen once" is the contract: a node is unobserved
// the moment it intersects, so scrolling back up never replays an entrance.

const gates = new Map();

const configKey = (threshold, rootMargin) => `${threshold}|${rootMargin}`;

/// Reveal `node` once it enters the viewport. `className` is added to the node
/// (pass null for a node whose entrance is driven some other way) and
/// `onReveal` runs with the node, for callers that animate it themselves.
/// Falls back to revealing immediately where IntersectionObserver is missing,
/// because the alternative is text that never unblurs.
export function observeReveal(node, options = {}) {
  if (!node) return;
  const className = options.className === undefined ? 'is-visible' : options.className;
  const onReveal = options.onReveal;
  const threshold = options.threshold ?? 0.14;
  const rootMargin = options.rootMargin || '0px 0px -8%';
  const IntersectionObserver = globalThis.IntersectionObserver;
  if (!IntersectionObserver) {
    if (className) node.classList.add(className);
    onReveal?.(node);
    return;
  }

  const key = configKey(threshold, rootMargin);
  let gate = gates.get(key);
  if (!gate) {
    const seen = new Map();
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const reveal = seen.get(entry.target);
        if (!reveal) continue;
        seen.delete(entry.target);
        observer.unobserve(entry.target);
        if (reveal.className) entry.target.classList.add(reveal.className);
        reveal.onReveal?.(entry.target);
      }
    }, { threshold, rootMargin });
    gate = { observer, seen };
    gates.set(key, gate);
  }
  gate.seen.set(node, { className, onReveal });
  gate.observer.observe(node);
}

/// Test/inspection hook: how many shared gates exist and how many nodes each is
/// still waiting on. A per-element observer would show up here as one gate per
/// node, which is exactly what this gate replaced.
export function revealState() {
  return [...gates.entries()].map(([key, gate]) => ({ config: key, pending: gate.seen.size }));
}
