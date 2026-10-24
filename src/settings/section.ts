/// One settings section: a tab-like title bar followed by a card. Pure HTML
/// so every page uses the same box model; the caller owns the control markup.
export interface SettingsSectionOptions {
  id?: string;
  title: string;
  /// i18n key for the title; when present the node keeps `data-i18n` so a
  /// language switch can retranslate it without rebuilding the shell.
  titleKey?: string;
  desc?: string;
  descKey?: string;
  children: string;
  className?: string;
}

export function escapeSettingText(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export function renderSection(options: SettingsSectionOptions): string {
  const id = options.id ? ` id="${escapeSettingText(options.id)}"` : "";
  const extra = options.className ? ` ${options.className}` : "";
  const titleKey = options.titleKey ? ` data-i18n="${escapeSettingText(options.titleKey)}"` : "";
  const descKey = options.descKey ? ` data-i18n="${escapeSettingText(options.descKey)}"` : "";
  const desc = options.desc || options.descKey
    ? `<span class="set-section__desc"${descKey}>${escapeSettingText(options.desc ?? "")}</span>`
    : "";
  return `<section${id} class="set-section${extra}">
    <header class="set-section__head">
      <h2 class="set-section__title"${titleKey}>${escapeSettingText(options.title)}</h2>
      ${desc}
    </header>
    <div class="set-section__card">${options.children}</div>
  </section>`;
}
