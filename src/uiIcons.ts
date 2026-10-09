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
import key from "./assets/icons/key-bold.svg?raw";
import magnifyingGlass from "./assets/icons/magnifying-glass-bold.svg?raw";
import bell from "./assets/icons/bell-bold.svg?raw";
import shield from "./assets/icons/shield-bold.svg?raw";
import globe from "./assets/icons/globe-bold.svg?raw";
import keyboard from "./assets/icons/keyboard-bold.svg?raw";
import arrowUp from "./assets/icons/arrow-up-bold.svg?raw";
import copy from "./assets/icons/copy-bold.svg?raw";
import trash from "./assets/icons/trash-bold.svg?raw";
import eye from "./assets/icons/eye-bold.svg?raw";
import eyeSlash from "./assets/icons/eye-slash-bold.svg?raw";
import pencil from "./assets/icons/pencil-simple-bold.svg?raw";
import plus from "./assets/icons/plus-bold.svg?raw";
import lock from "./assets/icons/lock-bold.svg?raw";
import lockOpen from "./assets/icons/lock-open-bold.svg?raw";

const ICONS = { gear, arrowsClockwise, shareNetwork, star, info, question, x, caretUp, caretDown, lightning, microphone, circleNotch, rows, palette, key, magnifyingGlass, bell, shield, globe, keyboard, arrowUp, copy, trash, eye, eyeSlash, pencil, plus, lock, lockOpen } as const;

export type UiIconName = keyof typeof ICONS;

/** Inline Phosphor SVG with the current text color and a stable UI class. */
export function uiIcon(name: UiIconName, label?: string): string {
  const title = label ? ` aria-label="${label.replace(/"/g, "&quot;")}"` : " aria-hidden=\"true\"";
  return ICONS[name]
    .replace("<svg ", `<svg class="ui-icon" focusable="false"${title} `)
    .replace(/\s(width|height)="16"/g, "");
}
