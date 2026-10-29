import type { Metadata } from "next";

import { LegalPage } from "@/components/legal-page";

export const metadata: Metadata = {
  title: "Terms and license",
  description: "Pane terms, the MIT licence and data notes.",
  alternates: { canonical: "https://pane.threetwoa.live/terms" },
};

export default function TermsPage() {
  return (
    <LegalPage
      title="Terms and license"
      updated="Last updated: 2026-10-26"
      authoritativeLabel="Authoritative license text"
      authoritativeHref="https://github.com/Aafff623/pane/blob/main/LICENSE"
      alternate={{ href: "/zh/terms", label: "中文" }}
    >
      <h2>License</h2>
      <p>
        Pane is released under the MIT license. You may use, copy, modify, merge, publish, distribute, sublicense and/or sell
        copies of the software, provided the copyright notice and this permission notice stay in the copies or substantial
        portions of the software.
      </p>

      <h2>Provided as is</h2>
      <p>
        The software is provided &quot;as is&quot;, without warranty of any kind, express or implied, including but not limited to
        the warranties of merchantability, fitness for a particular purpose and noninfringement. The authors or copyright holders
        are not liable for any claim, damages or other liability.
      </p>

      <h2>Third-party services</h2>
      <p>
        Pane reads credentials from your own machine and calls the APIs of the providers you enable. Each provider&apos;s use is
        governed by its own terms and privacy policy; Pane is not affiliated with any provider and makes no promise on their
        behalf.
      </p>

      <h2>Data and backups</h2>
      <p>
        Your local data is yours. Deleting %APPDATA%\PaneAafff623 or uninstalling removes Pane&apos;s local configuration and
        ledger — back up anything you want to keep.
      </p>

      <h2>Contact</h2>
      <p>
        Questions and licensing enquiries: <a href="https://github.com/Aafff623/pane/issues">GitHub Issues</a>.
      </p>
    </LegalPage>
  );
}
