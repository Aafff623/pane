const labelSelector = ".provider-name[data-fit-max], .overview-item-name";
const rowSelector = ".provider-head, .ovbar-line1, .overview-item-head";

// Use the flex layout's actual allocation: badges, icons and percentages
// already consume their own space. Never estimate glyph widths by character count.
export function fitDashboardTypography(root: HTMLElement): void {
  const dense = document.documentElement.dataset.density === "compact";
  for (const label of root.querySelectorAll<HTMLElement>(labelSelector)) {
    const row = label.closest<HTMLElement>(rowSelector);
    if (!row || row.clientWidth === 0) continue; // Collapsed/hidden groups fit when revealed.
    const overview = label.classList.contains("overview-item-name");
    const bars = !!label.closest(".overview-bar-item");
    const base = overview ? (bars ? 12 : 11) : Number(label.dataset.fitMax ?? 17);
    const referenceWidth = overview ? (bars ? 168 : 90) : 360;
    const ceiling = overview ? (bars ? 18 : 14) : 24;
    const minimum = overview ? (bars ? 11 : 10.5) : Math.max(12, Number(label.dataset.fitMin ?? 12));
    let size = Math.max(minimum, Math.min(ceiling, base * row.clientWidth / referenceWidth) - (dense ? 0.5 : 0));
    size = Math.floor(size * 2) / 2;
    label.style.fontSize = `${size}px`;
    while (size > minimum && label.scrollWidth > label.clientWidth) {
      size = Math.max(minimum, size - 0.5);
      label.style.fontSize = `${size}px`;
    }
    // CSS keeps ellipsis at the readable floor when the text still cannot fit.
  }
}

// Observe row widths rather than their heights, so our font changes cannot
// create a resize feedback loop. Reconcile observed nodes after each DOM render.
export function observeDashboardTypography(root: HTMLElement): () => void {
  let frame = 0;
  const schedule = () => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => fitDashboardTypography(root));
  };
  const widths = new WeakMap<Element, number>();
  const rows = new Set<Element>();
  const observer = new ResizeObserver((entries) => {
    let changed = false;
    for (const entry of entries) {
      const width = Math.round(entry.contentRect.width * 2) / 2;
      if (widths.get(entry.target) !== width) {
        widths.set(entry.target, width);
        changed = true;
      }
    }
    if (changed) schedule();
  });
  void document.fonts.ready.then(schedule);
  document.fonts.addEventListener("loadingdone", schedule);
  return () => {
    for (const row of rows) {
      if (!root.contains(row)) {
        observer.unobserve(row);
        rows.delete(row);
      }
    }
    for (const row of root.querySelectorAll(rowSelector)) {
      if (!rows.has(row)) {
        rows.add(row);
        observer.observe(row);
      }
    }
  };
}
