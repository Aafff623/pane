import gear from "./assets/icons/gear-bold.svg?raw";
import arrowsClockwise from "./assets/icons/arrows-clockwise-bold.svg?raw";
import shareNetwork from "./assets/icons/share-network-bold.svg?raw";
import star from "./assets/icons/star-bold.svg?raw";
import info from "./assets/icons/info-bold.svg?raw";
import question from "./assets/icons/question-bold.svg?raw";
import x from "./assets/icons/x-bold.svg?raw";
import caretUp from "./assets/icons/caret-up-bold.svg?raw";
import caretDown from "./assets/icons/caret-down-bold.svg?raw";
import lightning from "./assets/icons/lightning-bold.svg?raw";
import microphone from "./assets/icons/microphone-bold.svg?raw";
import circleNotch from "./assets/icons/circle-notch-bold.svg?raw";
import rows from "./assets/icons/rows-bold.svg?raw";
import palette from "./assets/icons/palette-bold.svg?raw";

const ICONS = { gear, arrowsClockwise, shareNetwork, star, info, question, x, caretUp, caretDown, lightning, microphone, circleNotch, rows, palette } as const;

export type UiIconName = keyof typeof ICONS;

/** Inline Phosphor SVG with the current text color and a stable UI class. */
export function uiIcon(name: UiIconName, label?: string): string {
  const title = label ? ` aria-label="${label.replace(/"/g, "&quot;")}"` : " aria-hidden=\"true\"";
  return ICONS[name]
    .replace("<svg ", `<svg class="ui-icon" focusable="false"${title} `)
    .replace(/\s(width|height)="16"/g, "");
}
