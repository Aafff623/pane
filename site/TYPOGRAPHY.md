# Website typography

- All website text uses variable **Inter**, including navigation, buttons, headings, code examples, the illustrative panel and its annotations. SVG icons and logo assets retain their original artwork.
- `next/font/google` downloads Inter at build time and serves the resulting WOFF2 files from `/_next/static/media/`. Visitors make no Google Fonts request. The root layout declares explicit Arial / Helvetica / sans-serif fallbacks.
- Website body, labels and buttons use Tailwind `text-base` (`1rem`, 16px with the default root font). CSS modules and prose use `--text-body`. Do not add responsive font-size overrides to body copy, reset the root font size, or substitute pixel-sized body text.
- Equivalent homepage section titles use `text-2xl md:text-4xl lg:text-5xl` (24 / 40 / 48px); the anatomy scene uses the same 24 / 48px endpoints within its existing sticky constraints. The closing statement remains a distinct display treatment.
- Keep the existing heading hierarchy: compact card labels at `text-base`, card/section titles at `text-xl` / `text-2xl`, and existing responsive page/display headings. The homepage headline retains its dedicated responsive treatment. Do not use display sizes for paragraphs or add new one-off heading sizes.
- Shared button padding remains unchanged; minimum heights preserve the existing 40px default, 32px small and 56px large controls. Homepage CTAs remain 56px on mobile and desktop.
- Illustration boundary: the panel replica's compact internal 9–11px type and the demo's editable target specimens retain their scale/layout fidelity. They still use Inter. Tutorial instructions, anatomy descriptions, annotation labels and grouping buttons are normal website copy at 16px.

## Rendered regression check

With a production build serving locally and Playwright available:

```sh
BASE_URL=http://localhost:3127 node website/scripts/check-typography.mjs
```

For an externally provisioned Playwright installation, set `PLAYWRIGHT_MODULE` to its absolute `index.mjs` path. `CHROME_EXECUTABLE` optionally selects an installed Chrome executable for a fresh isolated test browser. No website runtime dependency is required. `TYPOGRAPHY_OUTPUT` selects the evidence directory.

The check covers representative public templates at 320, 360, 400, 421, 480, 640 and 1440px; computed 16px body/buttons; Inter on visible text including code and the replica; headings; horizontal overflow; local font requests; image loading; expanded MCP accordions; panel stage controls; 56px hero targets; and rem scaling at a 200% root font. Screenshots and JSON measurements are emitted for review. This focused check is not a full accessibility-conformance audit.
