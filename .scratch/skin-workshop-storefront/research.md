# Agree with Docs · Research and settled decisions

2026-10-27. Source: user-confirmed conversation visuals and verified current repository materials.

## Evidence
- Screenshot of existing workshop: drawer-based market inside narrow float with huge nested preview and redundant tabs, user dissatisfied.
- Reference screenshot: desktop web App Store game detail with wide hero, summary fields, horizontal tall screenshot gallery.
- User-approved concept visuals: `assets/reference-gallery.png`, `assets/reference-detail.png`.
- GitHub `Aafff623/pane`: `docs/skin-market-design.md`; `.scratch/skin-workshop-round2/spec.md`; `src/skins.css`, `src/skins.ts` and `site/public/skins/index.json`.
- Related issues #49, #57. New #107 handles *presentation and IA only*.

## Decisions already agreed
- Put workshop in Settings → General → Appearance; do not build a new separate merchant app.
- Gallery of tall portrait previews; waterfall / regular grid toggle.
- Detail mirrors App Store's **information hierarchy** not its copyrighted graphics or metrics.
- Do not alter any of the three skin's internals; gallery mockups must be replaced by real component previews during integration.
- Preserve existing downloaded / active distinctions and native fallback.

## Unknown until working-tree inspection
- Exact current working-tree changes and uncommitted patches, deployment status of #49/#57, and availability of real screenshot capture hooks.
- Author metadata and banner assets in current official pack manifests (not exposed by public index). If absent, omit rather than invent.
