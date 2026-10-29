"use client";

import { useEffect, useState } from "react";

type Labels = { windows: string; macos: string };

type Target = { label: string; href: string };

const RELEASES = "https://github.com/Aafff623/pane/releases/latest";

function detectOS(): "windows" | "macos" | "linux" | "other" {
  const ua = navigator.userAgent;
  const data = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData?.platform ?? "";
  if (/windows|win32|win64/i.test(data) || /Windows/i.test(ua)) return "windows";
  if (/macos|macintosh|macintel/i.test(data) || /Macintosh|Mac OS X/i.test(ua)) return "macos";
  if (/linux|x11/i.test(data) || /Linux|X11/i.test(ua)) return /Android/i.test(ua) ? "other" : "linux";
  return "other";
}

async function detectArch(): Promise<"arm64" | "x64"> {
  try {
    const nav = navigator as Navigator & {
      userAgentData?: { getHighEntropyValues?: (h: string[]) => Promise<{ architecture?: string; bitness?: string }> };
    };
    const data = await nav.userAgentData?.getHighEntropyValues?.(["architecture", "bitness"]);
    if (data?.architecture === "arm") return "arm64";
    if (data?.architecture === "x86" && data?.bitness === "64") return "x64";
  } catch {}
  return /arm64|aarch64|armv8/i.test(navigator.userAgent) ? "arm64" : "x64";
}

let latestPromise: Promise<{ name: string; url: string }[]> | null = null;

function latestAssets() {
  latestPromise ??= fetch("/api/latest", { headers: { Accept: "application/json" } })
    .then((res) => (res.ok ? res.json() : null))
    .then((data) => (Array.isArray(data?.assets) ? data.assets : []))
    .catch(() => []);
  return latestPromise;
}

export function usePlatformTarget(labels: Labels): Target {
  const [target, setTarget] = useState<Target>({ label: labels.windows, href: "/download/windows" });

  useEffect(() => {
    let alive = true;
    (async () => {
      const params = new URLSearchParams(location.search);
      const osOverride = params.get("os");
      const os = osOverride === "windows" || osOverride === "macos" || osOverride === "linux" ? osOverride : detectOS();
      if (os !== "macos") return;
      const arch = params.get("arch") === "arm64" ? "arm64" : await detectArch();
      const assets = await latestAssets();
      const dmgs = assets.filter((asset) => /\.dmg$/i.test(asset.name || ""));
      const match = dmgs.find((asset) => (arch === "arm64" ? /arm64|aarch64/i.test(asset.name) : /(x64|x86_64|amd64)/i.test(asset.name))) ?? dmgs[0];
      if (!alive) return;
      setTarget({ label: labels.macos, href: match?.url ?? RELEASES });
    })();
    return () => {
      alive = false;
    };
  }, [labels.macos, labels.windows]);

  return target;
}
