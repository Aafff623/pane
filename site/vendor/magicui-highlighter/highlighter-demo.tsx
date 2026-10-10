// Source: https://github.com/magicuidesign/magicui/blob/ec1cce6c4192c0aaac279dd7e53537ccd5c99d44/apps/www/registry/example/highlighter-demo.tsx
// Revision: ec1cce6c4192c0aaac279dd7e53537ccd5c99d44
// Author: Dillion Verma / Magic UI
// License: MIT
// Path preserved from the original repository: apps/www/registry/example/highlighter-demo.tsx

import { Highlighter } from "@/registry/magicui/highlighter"

export default function HighlighterDemo() {
  return (
    <div className="text-center">
      <p className="leading-relaxed">
        The{" "}
        <Highlighter action="underline" color="#FF9800">
          Magic UI Highlighter
        </Highlighter>{" "}
        makes important{" "}
        <Highlighter action="highlight" color="#87CEFA">
          text stand out
        </Highlighter>{" "}
        effortlessly.
      </p>
    </div>
  )
}
