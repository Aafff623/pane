import { invoke } from "@tauri-apps/api/core";
// Only tiny catalog metadata is bundled, never font bytes or ZIP artifacts.
import officialCatalog from "../site/public/fonts/index.json";

export interface InstalledFontPack {
  id: string;
  name: string;
  version: string;
  family: string;
  license: string;
  entry_css: string;
}
interface FontPackOffer {
  id: string;
  name: string;
  family: string;
  version: string;
  size: number;
  url: string;
  sha256: string;
}
let installed: InstalledFontPack[] = [];
let loadInFlight: Promise<void> | null = null;

/** Load on app startup, before applying the saved uiFont; no font is bundled. */
export async function loadFontPacks(): Promise<void> {
  if (loadInFlight) return loadInFlight;
  loadInFlight = (async () => {
    const packs = await invoke<InstalledFontPack[]>("font_pack_list");
    let style = document.querySelector<HTMLStyleElement>("#pane-font-packs");
    if (!style) {
      style = document.createElement("style");
      style.id = "pane-font-packs";
      document.head.append(style);
    }
    style.textContent = packs.map((p) => p.entry_css).join("\n");
    installed = packs;
  })();
  try { await loadInFlight; } finally { loadInFlight = null; }
}

export function fontPackFamilies(): string[] {
  return [...new Set(installed.map((p) => p.family))];
}

export interface FontPackManagerOptions {
  t: (key: string, values?: Record<string, string | number>) => string;
  getSelected: () => string;
  /** Persist an empty uiFont when the removed family is currently selected. */
  onRemoved: (family: string) => void | Promise<void>;
  onChange?: () => void | Promise<void>;
}

/** Settings entry. Text uses textContent so imported metadata cannot add HTML. */
export async function mountFontPackManager(container: HTMLElement, options: FontPackManagerOptions): Promise<void> {
  const tr = (key: string) => options.t(`settings.fontPacks.${key}`);
  container.replaceChildren();
  const title = document.createElement("strong"); title.textContent = tr("title");
  title.dataset.i18n = "settings.fontPacks.title";
  const description = document.createElement("p"); description.textContent = tr("description");
  description.dataset.i18n = "settings.fontPacks.description";
  // MV Boli ships with every Windows install and Pane reads it straight off
  // the system font list, so the recommendation lives here as a note — the
  // font itself is never redistributed as a pack.
  const hint = document.createElement("p"); hint.className = "font-pack-hint";
  hint.textContent = tr("mvboliHint");
  hint.dataset.i18n = "settings.fontPacks.mvboliHint";
  const status = document.createElement("p"); status.setAttribute("role", "status");
  const retry = document.createElement("button");
  retry.type = "button";
  retry.className = "mini-btn";
  retry.textContent = tr("retry");
  retry.dataset.i18n = "settings.fontPacks.retry";
  retry.hidden = true;
  const statusRow = document.createElement("div");
  statusRow.className = "font-pack-status";
  statusRow.append(status, retry);
  const list = document.createElement("div");
  const imports = document.createElement("div");
  imports.style.cssText = "display:flex;gap:6px;flex-wrap:wrap;margin-top:8px";
  const url = document.createElement("input"); url.type = "url"; url.placeholder = tr("url"); url.setAttribute("aria-label", tr("url"));
  const digest = document.createElement("input"); digest.placeholder = tr("sha256"); digest.setAttribute("aria-label", tr("sha256")); digest.maxLength = 64;
  const importButton = document.createElement("button"); importButton.type = "button"; importButton.textContent = tr("import");
  importButton.dataset.i18n = "settings.fontPacks.import";
  url.dataset.i18nPlaceholder = url.dataset.i18nAria = "settings.fontPacks.url";
  digest.dataset.i18nPlaceholder = digest.dataset.i18nAria = "settings.fontPacks.sha256";
  imports.append(url, digest, importButton);
  container.append(title, description, hint, list, imports, statusRow);
  let offers: FontPackOffer[] = officialCatalog.packs;
  let busy = false;
  let catalogError = "";

  const run = async (action: () => Promise<void>) => {
    if (busy) return;
    busy = true; status.textContent = tr("loading"); render();
    try {
      await action();
      status.textContent = catalogError;
      if (!catalogError) status.removeAttribute("title");
    } catch (error) {
      console.error("[fontPacks]", error);
      status.textContent = tr("error");
      status.title = String(error);
    } finally {
      // A config callback can fail after the actual removal. Always refresh
      // installed state so a removed pack does not retain its injected CSS.
      try { await loadFontPacks(); await options.onChange?.(); }
      catch (error) {
        console.error("[fontPacks]", error);
        status.textContent = tr("error");
        status.title = String(error);
      }
      busy = false; render();
    }
  };
  const render = () => {
    list.replaceChildren();
    const ids = new Set(installed.map((p) => p.id));
    for (const pack of [...installed, ...offers.filter((p) => !ids.has(p.id))]) {
      const row = document.createElement("div");
      row.style.cssText = "display:flex;align-items:center;justify-content:space-between;gap:8px;margin-top:8px";
      const left = document.createElement("span");
      left.style.cssText = "display:flex;align-items:center;gap:6px;flex-wrap:wrap";
      const label = document.createElement("span");
      label.textContent = `${pack.name} · ${pack.version}${"size" in pack ? ` · ${Math.ceil(pack.size / 1024)} KB` : ""}`;
      if (ids.has(pack.id)) label.style.fontFamily = `"${pack.family}"`;
      left.append(label);
      // Catalog offers are Pane's curated picks; installed rows carry their state.
      if (!ids.has(pack.id)) {
        const badge = document.createElement("span");
        badge.className = "font-pack-badge";
        badge.textContent = tr("recommended");
        badge.dataset.i18n = "settings.fontPacks.recommended";
        left.append(badge);
      }
      const chip = document.createElement("span");
      chip.className = "font-pack-chip";
      chip.textContent = tr(ids.has(pack.id) ? "downloaded" : "available");
      chip.dataset.i18n = `settings.fontPacks.${ids.has(pack.id) ? "downloaded" : "available"}`;
      left.append(chip);
      const button = document.createElement("button"); button.type = "button";
      button.disabled = busy;
      button.textContent = tr(ids.has(pack.id) ? "remove" : "download");
      button.dataset.i18n = `settings.fontPacks.${ids.has(pack.id) ? "remove" : "download"}`;
      button.addEventListener("click", () => void run(async () => {
        if (ids.has(pack.id)) {
          await invoke("font_pack_remove", { id: pack.id });
          if (options.getSelected() === pack.family) await options.onRemoved(pack.family);
        } else {
          const offer = pack as FontPackOffer;
          await invoke("font_pack_download", { url: offer.url, sha256: offer.sha256 });
        }
      }));
      row.append(left, button); list.append(row);
    }
    if (!installed.length && !offers.length) { const empty = document.createElement("p"); empty.textContent = tr("empty"); list.append(empty); }
    importButton.disabled = busy;
    url.disabled = busy; digest.disabled = busy;
  };
  importButton.addEventListener("click", () => void run(async () => {
    await invoke("font_pack_download", { url: url.value.trim(), sha256: digest.value.trim() });
    url.value = ""; digest.value = "";
  }));
  const loadCatalog = async () => {
    retry.disabled = true;
    status.classList.remove("error");
    retry.hidden = true;
    status.textContent = tr("loading");
    try {
      // The demo lives under /pane-demo/ and serves its own tiny catalog;
      // the desktop bundle has no site assets, so it reads the published one.
      const catalogUrl = location.pathname.startsWith("/demo/")
        || location.pathname.startsWith("/pane-demo/")
        || location.hostname === "pane.threetwoa.live"
        ? "/fonts/index.json"
        : "https://pane.threetwoa.live/fonts/index.json";
      const response = await fetch(catalogUrl, { signal: AbortSignal.timeout(15000) });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data: unknown = await response.json();
      if (!data || typeof data !== "object" || !("packs" in data) || !Array.isArray(data.packs)) throw new Error("Invalid font catalog");
      offers = data.packs.filter((p): p is FontPackOffer => p && typeof p.id === "string" && typeof p.name === "string" && typeof p.family === "string" && typeof p.version === "string" && typeof p.size === "number" && typeof p.url === "string" && typeof p.sha256 === "string");
      catalogError = "";
      status.textContent = "";
      status.removeAttribute("title");
    } catch (error) {
      console.error("[fontPacks] catalog", error);
      catalogError = tr("catalogUnavailable");
      status.textContent = catalogError;
      status.title = String(error);
      status.classList.add("error");
      retry.hidden = false;
    } finally {
      retry.disabled = false;
      render();
    }
  };
  retry.addEventListener("click", () => void loadCatalog());
  status.textContent = tr("loading");
  try { await loadFontPacks(); } catch (error) {
    console.error("[fontPacks]", error);
    status.textContent = tr("error");
    status.title = String(error);
  }
  render();
  await loadCatalog();
}
