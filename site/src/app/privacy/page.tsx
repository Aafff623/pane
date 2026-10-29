import type { Metadata } from "next";

import { LegalPage } from "@/components/legal-page";

export const metadata: Metadata = {
  title: "Privacy",
  description: "Pane privacy: outbound requests, local storage and the telemetry boundary.",
  alternates: { canonical: "https://pane.threetwoa.live/privacy" },
};

export default function PrivacyPage() {
  return (
    <LegalPage
      title="Privacy"
      updated="Last updated: 2026-10-26"
      authoritativeLabel="Authoritative version"
      authoritativeHref="https://github.com/Aafff623/pane/blob/main/docs/privacy.md"
      alternate={{ href: "/zh/privacy", label: "中文" }}
    >
      <p>
        Pane is built on one rule: your data is nobody&apos;s business, including ours. There is no Pane account, and no Pane
        backend receives your quotas, spend, keys or provider data.
      </p>

      <h2>Every request that leaves your machine</h2>
      <ul>
        <li>A provider&apos;s own API: only for providers you enabled, sending that account&apos;s token/key exactly as its official tool would.</li>
        <li>Public price tables: LiteLLM, models.dev and the OpenUsage supplement — anonymous GET, no identifying data.</li>
        <li>Update check: this project&apos;s update/latest.json with a GitHub Releases fallback — anonymous GET, carrying no app data.</li>
        <li>Status page: only when a Claude card is stale or failed and the status probe is on — the public incident summary at status.claude.com.</li>
        <li>Your own Ollama: 127.0.0.1, only when enabled.</li>
      </ul>
      <p>No session recording, no event streams, no A/B flags, no autocapture and no telemetry. The app sends no statistics to anyone.</p>

      <h2>What is stored locally</h2>
      <p>
        Configuration, accounts, snapshots, the spend ledger, skins and font packs live in %APPDATA%\PaneAafff623. Secrets go to
        the OS credential store first and fall back to a protected local file only when that store is unavailable. Deleting those
        files erases Pane&apos;s local memory.
      </p>

      <h2>Telemetry</h2>
      <p>
        This build has no telemetry and therefore no toggle to switch off. If any reporting is ever added, it lands in the list
        above first and ships with an explicit control.
      </p>

      <h2>Contact</h2>
      <p>
        Questions and data requests: <a href="https://github.com/Aafff623/pane/issues">GitHub Issues</a>.
      </p>
    </LegalPage>
  );
}
