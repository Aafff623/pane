import type { Metadata } from "next";

import { LegalPage } from "@/components/legal-page";
import { SITE_FACTS } from "@/content/facts";

export const metadata: Metadata = {
  title: "Privacy",
  description: "Pane privacy: outbound requests, local storage and the telemetry boundary.",
  alternates: { canonical: "/en/privacy/", languages: { "zh-CN": "/privacy/", en: "/en/privacy/" } },
};

export default function PrivacyPage() {
  return (
    <LegalPage
      lang="en"
      title="Privacy"
      updated="Last updated: 2026-10-26"
      authoritativeHref={`${SITE_FACTS.repo}/blob/main/docs/privacy.md`}
      authoritativeLabel="Authoritative version"
      alternate={{ href: "/privacy/", label: "中文" }}
    >
      <p>
        Pane is built on one rule: your data is nobody&apos;s business, including ours. There is no Pane account, and no Pane
        backend receives your quotas, spend, keys or provider data.
      </p>

      <h2>Every request that leaves your machine</h2>
      <ul>
        <li>
          <strong>A provider&apos;s own API</strong>: only for providers you enabled, sending that account&apos;s token/key
          exactly as its official tool would.
        </li>
        <li>
          <strong>Public price tables</strong>: LiteLLM, models.dev and the OpenUsage supplement — anonymous GET, no identifying
          data.
        </li>
        <li>
          <strong>Update check</strong>: this project&apos;s <code>update/latest.json</code> with a GitHub Releases fallback —
          anonymous GET, carrying no app data.
        </li>
        <li>
          <strong>Status page</strong>: only when a Claude card is stale or failed and the status probe is on — an anonymous GET
          of the public incident summary at <code>status.claude.com</code>.
        </li>
        <li>
          <strong>Your own Ollama</strong>: <code>127.0.0.1</code>, only when enabled.
        </li>
      </ul>
      <p>No session recording, no event streams, no A/B flags, no autocapture and no telemetry. The app sends no statistics to anyone.</p>

      <h2>What is stored locally</h2>
      <p>
        Configuration, accounts, snapshots, the spend ledger, skins and font packs live in <code>%APPDATA%\PaneAafff623</code>.
        Secrets go to the OS credential store first and fall back to a protected local file only when that store is unavailable.
        Deleting those files erases Pane&apos;s local memory.
      </p>

      <h2>Telemetry</h2>
      <p>
        This build has no telemetry and therefore no toggle to switch off. If any reporting is ever added, it lands in the list
        above first and ships with an explicit control.
      </p>

      <h2>Contact</h2>
      <p>
        Questions and data requests:{" "}
        <a href={SITE_FACTS.issues} target="_blank" rel="noopener noreferrer">
          GitHub Issues
        </a>
        .
      </p>
    </LegalPage>
  );
}
