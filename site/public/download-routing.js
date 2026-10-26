// Platform routing for the download surface (#85). The visitor's OS decides
// which CTA is primary and which asset the button points at; the edge worker
// still owns the Windows redirect, and the static fallback stays usable when
// /api/latest cannot answer. Linux builds are not promoted yet: the site keeps
// the planned badge and leaves the Windows CTA in place.
// Classic scripts share one global lexical scope: another file's `const root`
// would make this whole script fail to parse. Keep every binding local.
(() => {
const RELEASES_PAGE = "https://github.com/Aafff623/pane/releases/latest";
const COPY = {
  zh: { windows: "下载 Windows 版", macos: "下载 macOS 版 (Beta)", latest: "最新发布" },
  en: { windows: "Download for Windows", macos: "Download for macOS (Beta)", latest: "Latest release" },
};

const root = document.documentElement;
let platform = "other";
let arch = "x64";
let latestHref = null;

const copy = () => (root.lang === "en" ? COPY.en : COPY.zh);
const platformName = () =>
  platform === "macos" ? "macOS" : platform === "windows" ? "Windows" : platform === "linux" ? "Linux" : "Desktop";

function detectOS() {
  const ua = navigator.userAgent || "";
  const data = navigator.userAgentData?.platform || "";
  const nav = navigator.platform || "";
  const ipad = /Macintosh/i.test(ua) && navigator.maxTouchPoints > 1;
  if (ipad) return "other";
  if (/windows|win32|win64/i.test(data) || /Windows/i.test(ua)) return "windows";
  if (/macos|macintosh|macintel/i.test(data) || /Macintosh|Mac OS X/i.test(ua)) return "macos";
  if (/linux|x11/i.test(data) || /Linux|X11/i.test(ua)) return /Android/i.test(ua) ? "other" : "linux";
  return "other";
}

function detectArch() {
  return /arm64|aarch64|armv8/i.test(navigator.userAgent || "") ? "arm64" : "x64";
}

async function refineArch() {
  try {
    const data = await navigator.userAgentData?.getHighEntropyValues?.(["architecture", "bitness"]);
    if (data?.architecture === "arm") return "arm64";
    if (data?.architecture === "x86" && data?.bitness === "64") return "x64";
  } catch {}
  return arch;
}

const isArm = (name) => /(arm64|aarch64)/i.test(name);
const isX64 = (name) => /(x64|x86_64|amd64)/i.test(name);

function pickAsset(assets) {
  if (!Array.isArray(assets)) return null;
  if (platform === "macos") {
    const dmgs = assets.filter((item) => /\.dmg$/i.test(item?.name || ""));
    return dmgs.find((item) => (arch === "arm64" ? isArm(item.name) : isX64(item.name))) || dmgs[0] || null;
  }
  if (platform === "linux") {
    const packages = assets.filter((item) => /\.(appimage|deb)$/i.test(item?.name || ""));
    return packages.find((item) => (arch === "arm64" ? isArm(item.name) : isX64(item.name))) || packages[0] || null;
  }
  if (platform === "windows") {
    const exes = assets.filter((item) => /-setup\.exe$/i.test(item?.name || ""));
    return exes.find((item) => (arch === "arm64" ? isArm(item.name) : !isArm(item.name))) || null;
  }
  return null;
}

function applyPlatform() {
  root.dataset.os = platform;
  root.dataset.arch = arch;
  const text = copy();
  const label = platform === "macos" ? text.macos : text.windows;
  const href = latestHref || (platform === "macos" ? RELEASES_PAGE : "/download/windows");
  for (const link of document.querySelectorAll("[data-latest-installer]")) {
    link.setAttribute("href", href);
    const node = link.querySelector("[data-download-label]");
    if (node) node.textContent = label;
  }
  const actions = document.querySelector(".bottom-actions");
  const mac = actions?.querySelector('[data-platform="macos"]');
  const win = actions?.querySelector('[data-platform="windows"]');
  if (mac && win) {
    const macPrimary = platform === "macos";
    mac.dataset.variant = macPrimary ? "primary" : "tertiary";
    win.dataset.variant = macPrimary ? "tertiary" : "primary";
    if (macPrimary) mac.setAttribute("href", latestHref || RELEASES_PAGE);
  }
  const prefix = document.querySelector("[data-release-prefix]");
  if (prefix) prefix.textContent = text.latest;
  const platformNode = document.querySelector("[data-release-platform]");
  if (platformNode) platformNode.textContent = `${platformName()} · ${arch}`;
}

async function loadLatest() {
  try {
    const res = await fetch("/api/latest", { headers: { Accept: "application/json" } });
    if (!res.ok) return;
    const data = await res.json();
    const version = typeof data?.version === "string" ? data.version.replace(/^v/, "") : "";
    const versionNode = document.querySelector("[data-latest-version]");
    if (version && versionNode) versionNode.textContent = `v${version}`;
    const asset = pickAsset(data?.assets);
    if (!asset) return;
    if (platform === "macos" || (platform === "windows" && arch === "arm64")) {
      latestHref = asset.url;
      applyPlatform();
    }
  } catch {}
}

(async () => {
  // ?os=macos&arch=arm64 is the acceptance hook: it previews another platform
  // without changing the browser's user agent.
  const params = new URLSearchParams(location.search);
  const osOverride = params.get("os");
  platform = ["windows", "macos", "linux"].includes(osOverride) ? osOverride : detectOS();
  const archOverride = params.get("arch");
  arch = archOverride === "arm64" || archOverride === "x64" ? archOverride : await refineArch();
  applyPlatform();
  await loadLatest();
  document.addEventListener("DOMContentLoaded", applyPlatform, { once: true });
})();
})();
