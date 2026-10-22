/** Skin appearance only. Provider data and user fonts stay intact. */
import { invoke } from "@tauri-apps/api/core";

/// A skin id: any installed resource-pack id. `signal` / `ink` are ordinary
/// official packs now — they ship through the workshop, not the app bundle.
export type SkinId = string;
export const SKIN_STORAGE_KEY = "pane.skin.id";

/// Shape check shared with Rust `valid_skin_id`: a pack id may be persisted
/// before the pack exists locally (the index migration installs it).
const SKIN_ID_RE = /^[a-z0-9][a-z0-9-]{0,31}$/;

export function validSkinId(id: unknown): id is string {
  return typeof id === "string" && SKIN_ID_RE.test(id);
}

/// Installed resource-pack skins (workshop). Packs arrive through the
/// workshop and hot-load without a restart. The entry CSS arrives fully
/// self-contained from Rust (assets inlined as data URLs), so injecting one
/// <style> is the whole install — and the share-PNG pipeline needs no pack
/// awareness beyond URL-valued custom properties.
export interface PackSkin {
  id: string;
  name: string;
  version: string;
  entryCss: string;
}

let packSkins: PackSkin[] = [];

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
      (p) => validSkinId(p.id) && typeof p.url === "string" && typeof p.sha256 === "string",
    );
    try {
      localStorage.setItem("pane.skins.index", JSON.stringify(remoteIndex));
    } catch { /* private mode — memory copy is enough for the session. */ }
  } catch {
    if (!remoteIndex.length) {
      try {
        const cached = localStorage.getItem("pane.skins.index");
        if (cached) remoteIndex = JSON.parse(cached) as RemotePackEntry[];
      } catch { /* nothing cached — workshop falls back to the empty state. */ }
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
/// demo) — the workshop then shows only index entries.
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
  for (const p of packSkins) injectPackCss(p);
}

export function normalizeSkin(id: unknown): SkinId | null {
  return validSkinId(id) ? id : null;
}

/// Early-paint cache only; config.json is authoritative. A shaped id is kept
/// even before the pack registry settles (the migration downloads it), so an
/// installed skin never flashes native on boot.
export function readSkin(storage: Pick<Storage, "getItem" | "removeItem">): SkinId | null {
  try {
    const id = storage.getItem(SKIN_STORAGE_KEY);
    if (!id) return null;
    if (validSkinId(id)) return id;
    storage.removeItem(SKIN_STORAGE_KEY);
  } catch { /* Storage unavailable: keep the native appearance. */ }
  return null;
}

/// Installed pack CSS arrives with every asset already inlined by Rust, so
/// there is nothing external left to embed. Signal's post-icon crop reads
/// URL-valued custom properties at use sites; Chromium's SVG rasterizer does
/// not resolve those reliably, so inline the computed values into the clone.
export async function embedSkinImages(markup: string, skin: SkinId): Promise<string> {
  if (skin !== "signal") return markup;
  try {
    const style = getComputedStyle(document.documentElement);
    for (const name of ["--post-atlas", "--post-atlas-extra"]) {
      const value = style.getPropertyValue(name).trim();
      if (value) markup = markup.split(`var(${name})`).join(value);
    }
  } catch { /* Native appearance: no pack variables to inline. */ }
  return markup;
}