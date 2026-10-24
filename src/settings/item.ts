import { escapeSettingText } from "./section.ts";

/// One row inside a section: a 14/600 label, an optional 12/400 description,
/// and one control. `stacked` puts the control on its own full-width line.
export interface SettingsItemOptions {
  label: string;
  /// i18n keys keep the row translatable after a language switch without a
  /// full shell rebuild; plain text is for dynamic values only.
  labelKey?: string;
  desc?: string;
  descKey?: string;
  control: string;
  stacked?: boolean;
  /// Defaults to the encoded label. Pages that reuse a label must pass a
  /// unique anchor so search and the two-way coverage check stay exact.
  anchor?: string;
  className?: string;
}

export function renderItem(options: SettingsItemOptions): string {
  const anchor = options.anchor ?? encodeURIComponent(options.label);
  const labelKey = options.labelKey
    ? ` data-i18n="${escapeSettingText(options.labelKey)}"`
    : "";
  const descKey = options.descKey ? ` data-i18n="${escapeSettingText(options.descKey)}"` : "";
  const desc = options.desc || options.descKey
    ? `<span class="set-row__desc"${descKey}>${escapeSettingText(options.desc ?? "")}</span>`
    : "";
  const extra = options.className ? ` ${options.className}` : "";
  return `<div class="set-row${options.stacked ? " set-row--stacked" : ""}${extra}" data-setting-anchor="${escapeSettingText(anchor)}">
    <div class="set-row__text">
      <span class="set-row__label"${labelKey}>${escapeSettingText(options.label)}</span>
      ${desc}
    </div>
    <div class="set-row__control">${options.control}</div>
  </div>`;
}
