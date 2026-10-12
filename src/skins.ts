/** Built-in skins own appearance only. Provider data and user fonts stay intact. */
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
export type SkinId = "signal" | "ink";
export const SKIN_STORAGE_KEY = "pane.skin.id";
export const SKINS = [
  { id: "signal" as const, nameKey: "skin.signal" as const, taglineKey: "skin.signalTagline" as const },
  { id: "ink" as const, nameKey: "skin.ink" as const, taglineKey: "skin.inkTagline" as const },
];
export function normalizeSkin(id: unknown): SkinId | null {
  return id === "signal" || id === "ink" ? id : null;
}

export function readSkin(storage: Pick<Storage, "getItem" | "removeItem">): SkinId | null {
  try {
    const id = storage.getItem(SKIN_STORAGE_KEY);
    if (normalizeSkin(id)) return normalizeSkin(id);
    // Retired experimental skins must never reappear after an upgrade.
    if (id) storage.removeItem(SKIN_STORAGE_KEY);
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
