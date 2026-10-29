"use client";

import { useEffect, useState } from "react";

import styles from "./demo.module.scss";

// Detects whether the Design Mode extension is active on this page by
// watching for the override stylesheet (`<style id="dm-applied-styles">`)
// that the content script injects when the side panel attaches, OR for
// any element with a `data-dm` attribute (which is set on every layer the
// extension has assigned an id to).
//
// Uses a MutationObserver so the pill flips green the instant the user
// activates the panel — no polling, no setInterval.
export function ExtensionDetected() {
  const [detected, setDetected] = useState(false);

  useEffect(() => {
    function check(): boolean {
      if (document.getElementById("dm-applied-styles")) return true;
      if (document.querySelector("[data-dm]")) return true;
      return false;
    }

    if (check()) {
      setDetected(true);
      return;
    }

    const observer = new MutationObserver(() => {
      if (check()) {
        setDetected(true);
        observer.disconnect();
      }
    });
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["data-dm"],
    });
    return () => observer.disconnect();
  }, []);

  return (
    <div
      className={styles.extensionBanner}
      data-detected={detected ? "true" : "false"}
      role="status"
      aria-live="polite"
    >
      <span className={styles.extensionDot} aria-hidden="true" />
      <span>
        {detected ? "Extension detected" : "Open the side panel to begin"}
      </span>
    </div>
  );
}
