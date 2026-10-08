// pane.threetwoa.live. The site is static files in public/; wrangler.jsonc
// sets run_worker_first, so every request lands here and is handed straight
// to the asset binding. (The workers.dev trial address is off: wrangler
// disables it once a custom-domain route is declared.)
//
// Language: the site ships Chinese only. The bilingual edge rewrite
// (/<lang>/ + Accept-Language negotiation, dictionaries in i18n.js) was
// built and then pulled before launch — do English later. i18n.js is parked
// with the dictionaries and the page's data-i18n markers so that pass starts
// from working material instead of from scratch.
//
// TODO (Phase 2): /api/latest — proxy GitHub's releases/latest API for
// Aafff623/pane into {version, url, published, assets:{name:{url,size,sha256}}}
// (the release workflow already publishes latest.json + sha256 for the app's
// updater; mirror that shape). Phase 1 resolves the newest Windows installer
// in the browser from the public API, with the releases page as fallback.
//
// No analytics beacon: Magpie injects Cloudflare Web Analytics here, but that
// needs a token tied to a zone we don't have yet. Add it when a real domain
// lands — one edge function, no markup change.

export default {
  async fetch(req, env) {
    return env.ASSETS.fetch(req);
  },
};
