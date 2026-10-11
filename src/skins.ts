/** Built-in skins own appearance only. Provider data and user fonts stay intact. */
import day from "./assets/skins/cloud-post/day.png";
import night from "./assets/skins/cloud-post/night.png";
import icons from "./assets/skins/cloud-post/icons.png";
import extra from "./assets/skins/cloud-post/extra.png";
import mascot from "./assets/skins/cloud-post/mascot.png";
export type SkinId = "signal";
export const SKIN_STORAGE_KEY = "pane.skin.id";
export const SKINS = [{ id: "signal" as const, nameKey: "skin.signal", taglineKey: "skin.signalTagline" }];

export function readSkin(storage: Pick<Storage, "getItem" | "removeItem">): SkinId | null {
  try {
    const id = storage.getItem(SKIN_STORAGE_KEY);
    if (id === "signal") return id;
    // Retired experimental skins must never reappear after an upgrade.
    if (id) storage.removeItem(SKIN_STORAGE_KEY);
  } catch { /* Storage unavailable: keep the native appearance. */ }
  return null;
}

const embeddedImages = new Map<string, Promise<string>>();

// SVG foreignObject rasterization cannot fetch external CSS images. Embed only
// the bundled theme assets, never provider URLs or account data. Cache successes
// across shares and allow a failed local read to be retried on the next share.
export async function embedSkinImages(markup: string): Promise<string> {
  for (const asset of [day, night, icons, extra, mascot]) {
    if (!markup.includes(asset)) continue;
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
    markup = markup.split(asset).join(await data);
  }
  return markup;
}
