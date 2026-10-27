// The site's comic-pixel skin is a preview of the in-app skin, not a second
// theme system: it only flips data-site-skin, and every comic rule lives under
// that attribute in site.css. The pre-paint bootstrap in index.html already
// applies the stored/query value; this file keeps the header control honest.
(() => {
const STORAGE_KEY = "pane-site-skin";
const root = document.documentElement;
const button = document.querySelector("#site-skin");

const label = () => {
  const english = root.lang === "en";
  const text = english ? "Comic skin" : "漫画皮肤";
  button?.setAttribute("aria-label", text);
  button?.setAttribute("title", text);
};

const apply = (skin, persist) => {
  root.dataset.siteSkin = skin === "comic-pixel" ? "comic-pixel" : "native";
  button?.setAttribute("aria-pressed", String(root.dataset.siteSkin === "comic-pixel"));
  if (persist) {
    try {
      localStorage.setItem(STORAGE_KEY, root.dataset.siteSkin);
    } catch {}
  }
  label();
};

apply(root.dataset.siteSkin, false);
document.addEventListener("DOMContentLoaded", label, { once: true });
button?.addEventListener("click", () => {
  apply(root.dataset.siteSkin === "comic-pixel" ? "native" : "comic-pixel", true);
});
})();
