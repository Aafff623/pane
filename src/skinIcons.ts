import icons from "./assets/skins/cloud-post/icons.png";
import extra from "./assets/skins/cloud-post/extra.png";

// Crop the original hand-drawn atlas in CSS. Its rows are not perfectly uniform;
// measured bounds keep neighbouring drawings out of small controls.
export function postIcon(index: number, supplement = false, label?: string): string {
  const cols = supplement ? 4 : 6;
  const col = index % cols, row = Math.floor(index / cols);
  const imageWidth = supplement ? 1280 : 1536, imageHeight = supplement ? 1280 : 1024;
  const x = supplement ? col * 320 + 16 : [20, 296, 545, 790, 1015, 1285][col];
  const y = supplement ? row * 320 + 16 : [44, 299, 537, 743][row];
  const width = supplement ? 288 : [259, 216, 222, 216, 261, 230][col];
  const height = supplement ? 288 : [244, 233, 204, 232][row];
  const size = Math.max(width, height);
  const aria = label ? `role="img" aria-label="${label.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;")}"` : 'aria-hidden="true"';
  return `<span class="post-icon" ${aria} style="--post-w:${width / size};--post-h:${height / size};--post-size:${imageWidth / width * 100}% ${imageHeight / height * 100}%;--post-position:${x / (imageWidth - width) * 100}% ${y / (imageHeight - height) * 100}%"><span class="post-icon-art" aria-hidden="true" style="background-image:url('${supplement ? extra : icons}')"></span></span>`;
}

export const POST_ICON_MAP: Record<string, readonly [number, boolean?]> = {
  gear: [2], arrowsClockwise: [3], shareNetwork: [21], star: [0, true],
  info: [1, true], question: [11], x: [18], caretUp: [2, true],
  caretDown: [3, true], lightning: [4, true], microphone: [5, true],
  circleNotch: [23], rows: [5], palette: [4], key: [6],
  magnifyingGlass: [12], bell: [8], shield: [7], globe: [6, true],
  keyboard: [7, true], arrowUp: [2, true], copy: [10], trash: [9],
  eye: [8, true], eyeSlash: [9, true], pencil: [10, true], plus: [11, true],
  lock: [12, true], lockOpen: [13, true],
};
