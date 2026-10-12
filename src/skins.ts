/** Built-in skins own appearance only. Provider data and user fonts stay intact. */
import { invoke } from "@tauri-apps/api/core";
import day from "./assets/skins/cloud-post/day.png";
import night from "./assets/skins/cloud-post/night.png";
import icons from "./assets/skins/cloud-post/icons.png";
import extra from "./assets/skins/cloud-post/extra.png";
import mascot from "./assets/skins/cloud-post/mascot.png";
import koi from "./assets/skins/ink-wash/koi-red-v4.png";
import mountains from "./assets/skins/ink-wash/mountains-v4.png";
import grain from "./assets/skins/ink-wash/paper-grain.svg";
import brush from "./assets/skins/ink-wash/brush-mask.svg";
import chineseBrush from "./assets/skins/ink-wash/fonts/MaShanZheng-ui.ttf?url";
import latinBrush from "./assets/skins/ink-wash/fonts/PermanentMarker-Regular.ttf?url";
/// A skin id: the two built-ins, or any installed resource-pack id.
export type SkinId = "signal" | "ink" | (string & {});
export const SKIN_STORAGE_KEY = "pane.skin.id";
export const SKINS = [
  { id: "signal" as const, nameKey: "skin.signal" as const, taglineKey: "skin.signalTagline" as const },
  { id: "ink" as const, nameKey: "skin.ink" as const, taglineKey: "skin.inkTagline" as const },
];

/// Installed resource-pack skins (workshop). Built-ins ship with the app;
/// packs arrive through the workshop and hot-load without a restart. The
/// entry CSS arrives fully self-contained from Rust (assets inlined as
/// data URLs), so injecting one <style> is the whole install — and the
/// share-PNG pipeline needs no pack awareness.
export interface PackSkin {
  id: string;
  name: string;
  version: string;
  entryCss: string;
}

let packSkins: PackSkin[] = [];
let packsLoaded = false;

/// The official distribution index (static file beside the site deploy —
/// same origin, same pipeline, no extra infrastructure). Last good copy
/// survives in localStorage so the workshop works offline.
export const SKINS_INDEX_URL = "https://pane.threetwoa.live/skins/index.json";
export interface RemotePackEntry {
  id: string;
  name: string;
  version: string;
  size: number;
  sha256: string;
  url: string;
}
let remoteIndex: RemotePackEntry[] = [];

export function remotePackIndex(): RemotePackEntry[] {
  return remoteIndex;
}

export async function refreshSkinIndex(): Promise<void> {
  try {
    const res = await fetch(SKINS_INDEX_URL, { cache: "no-store" });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const doc = (await res.json()) as { packs?: RemotePackEntry[] };
    remoteIndex = (doc.packs ?? []).filter(
      (p) => typeof p.id === "string" && typeof p.url === "string" && typeof p.sha256 === "string",
    );
    try {
      localStorage.setItem("pane.skins.index", JSON.stringify(remoteIndex));
    } catch { /* private mode — memory copy is enough for the session. */ }
  } catch {
    if (!remoteIndex.length) {
      try {
        const cached = localStorage.getItem("pane.skins.index");
        if (cached) remoteIndex = JSON.parse(cached) as RemotePackEntry[];
      } catch { /* nothing cached — workshop just shows built-ins. */ }
    }
  }
}

export function packSkinList(): PackSkin[] {
  return packSkins;
}

export function isPackSkin(id: string | null | undefined): boolean {
  return !!id && packSkins.some((p) => p.id === id);
}

function injectPackCss(p: PackSkin): void {
  if (document.querySelector(`style[data-skin-pack="${p.id}"]`)) return;
  const style = document.createElement("style");
  style.dataset.skinPack = p.id;
  style.textContent = p.entryCss;
  document.head.appendChild(style);
}

/// Loads the installed pack registry and injects each pack's stylesheet.
/// Safe to call twice; safe when the command surface is missing (browser
/// demo) — built-ins only, then.
export async function refreshInstalledPacks(): Promise<void> {
  let packs: { id: string; name: string; version: string; entry_css: string }[];
  try {
    packs = await invoke("skin_pack_list");
  } catch {
    return;
  }
  for (const old of packSkins) {
    document.querySelector(`style[data-skin-pack="${old.id}"]`)?.remove();
  }
  packSkins = packs.map((p) => ({ id: p.id, name: p.name, version: p.version, entryCss: p.entry_css }));
  packsLoaded = true;
  for (const p of packSkins) injectPackCss(p);
}

export function normalizeSkin(id: unknown): SkinId | null {
  if (id === "signal" || id === "ink") return id;
  return typeof id === "string" && packSkins.some((p) => p.id === id) ? id : null;
}

export function readSkin(storage: Pick<Storage, "getItem" | "removeItem">): SkinId | null {
  try {
    const id = storage.getItem(SKIN_STORAGE_KEY);
    const known = normalizeSkin(id);
    if (known) return known;
    // Retired built-ins must never reappear after an upgrade. An id that
    // could be a pack keeps its early-paint cache until the pack registry
    // settles — config.json stays authoritative either way.
    if (id && packsLoaded && !isPackSkin(id)) storage.removeItem(SKIN_STORAGE_KEY);
  } catch { /* Storage unavailable: keep the native appearance. */ }
  return null;
}

const embeddedImages = new Map<string, Promise<string>>();

// SVG foreignObject rasterization cannot fetch external CSS images. Embed only
// the bundled theme assets, never provider URLs or account data. Cache successes
// across shares and allow a failed local read to be retried on the next share.
export async function embedSkinImages(markup: string, skin: SkinId): Promise<string> {
  const assets = skin === "ink"
    ? [koi, mountains, grain, brush, chineseBrush, latinBrush]
    : [day, night, icons, extra, mascot];
  for (const asset of assets) {
    // CSSOM resolves stylesheet URLs to absolute URLs, while Vite imports
    // remain root-relative. Replace the whole absolute URL first, otherwise
    // the origin would be left before the data URI (http://…data:image/…).
    const urls = [...new Set([new URL(asset, document.baseURI).href, asset])];
    if (!urls.some(url => markup.includes(url))) continue;
    let data = embeddedImages.get(asset);
    if (!data) {
      data = fetch(asset).then(async response => {
        if (!response.ok) throw new Error("Could not read bundled skin image");
        const blob = await response.blob();
        return new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result));
          reader.onerror = () => reject(new Error("Could not embed bundled skin image"));
          reader.readAsDataURL(blob);
        });
      }).catch(error => { embeddedImages.delete(asset); throw error; });
      embeddedImages.set(asset, data);
    }
    for (const url of urls) markup = markup.split(url).join(await data);
  }
  return markup;
}
