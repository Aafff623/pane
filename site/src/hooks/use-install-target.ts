"use client";

import { useSyncExternalStore } from "react";

export type InstallTarget = "chrome" | "firefox" | "safari";

// Pure UA → target mapping. Firefox (desktop/Android) gets AMO. Safari is
// "Safari" in the UA without any Chromium/Chrome/iOS-Chrome/iOS-Firefox/Edge
// marker, since every Chromium UA also contains "Safari". Everything else —
// Chrome, Edge, Brave, Arc, unknown — falls back to the Chrome Web Store.
export function detectInstallTarget(userAgent: string): InstallTarget {
  if (/firefox/i.test(userAgent)) return "firefox";
  if (
    /safari/i.test(userAgent) &&
    !/chrome|chromium|crios|fxios|edg/i.test(userAgent)
  ) {
    return "safari";
  }
  return "chrome";
}

const subscribe = () => () => {};
const getClientSnapshot = (): InstallTarget =>
  detectInstallTarget(navigator.userAgent);
// Server render and hydration both use "chrome": the majority target and a
// working link for no-JS clients and crawlers. React then re-renders with the
// client snapshot, so there is no hydration mismatch.
const getServerSnapshot = (): InstallTarget => "chrome";

export function useInstallTarget(): InstallTarget {
  return useSyncExternalStore(subscribe, getClientSnapshot, getServerSnapshot);
}
