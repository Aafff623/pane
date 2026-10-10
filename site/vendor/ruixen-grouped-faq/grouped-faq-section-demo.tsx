// Source: https://github.com/ruixenui/ruixen.com/blob/a017388715c0d63b357cc6d56944784dfb696a5f/registry/example/grouped-faq-section-demo.tsx
// Revision: a017388715c0d63b357cc6d56944784dfb696a5f
// Author: Ruixen UI
// License: MIT
// Path preserved from the original repository: registry/example/grouped-faq-section-demo.tsx

"use client";

import GroupedFAQSection from "../ruixenui/grouped-faq-section";

export default function GroupedFAQSectionDemo() {
  return (
    <GroupedFAQSection
      contactHref="https://github.com/ruixenui/ruixen.com/issues"
      contactCTA="open a GitHub issue"
      className="py-0 md:py-0"
    />
  );
}
