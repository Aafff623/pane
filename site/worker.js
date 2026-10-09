// pane.threetwoa.live. The site is static files in public/; wrangler.jsonc
// sets run_worker_first, so every request lands here first.
//
//   /download/windows   the newest Windows installer. Resolved here, at the
//                       edge, so a visitor only ever talks to this origin:
//                       api.github.com is slow or unreachable from some
//                       networks, while the worker reaches it from
//                       Cloudflare. Cached ten minutes; any failure falls
//                       back to the releases page, so the button always
//                       leads somewhere useful. The download buttons in
//                       public/index.html point at this route — no client
//                       script involved.
//
// Everything else is the static site in public/, via env.ASSETS.fetch.
// (The workers.dev trial address is off: wrangler disables it once a
// custom-domain route is declared.)
//
// Language: the site ships Chinese only. The bilingual edge rewrite
// (/<lang>/ + Accept-Language negotiation, dictionaries in i18n.js) was
// built and then pulled before launch — do English later. i18n.js is parked
// with the dictionaries and the page's data-i18n markers so that pass starts
// from working material instead of from scratch.
//
// No analytics beacon: Magpie injects Cloudflare Web Analytics here, but that
// needs a token tied to a zone we don't have yet. Add it when a real domain
// lands — one edge function, no markup change.

const REPO = "Aafff623/pane";
const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;
const INSTALLER = /-setup\.exe$/;

// latestWindowsInstaller asks GitHub for the newest release's Windows
// installer. The cf.cacheTtl keeps the API call off the request path for
// repeat visitors (GitHub allows 60 anonymous calls an hour per IP).
async function latestWindowsInstaller() {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json", "User-Agent": "pane-site" },
    cf: { cacheTtl: 600, cacheEverything: true },
  });
  if (!res.ok) return null;
  const release = await res.json();
  const asset = (release.assets || []).find((item) => INSTALLER.test(item.name || ""));
  return asset?.browser_download_url ?? null;
}

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (url.pathname === "/download/windows") {
      let target = RELEASES_PAGE;
      try {
        target = (await latestWindowsInstaller()) || RELEASES_PAGE;
      } catch {
        // Unreachable API, bad JSON, missing asset: the releases page still
        // gets the visitor to the right download.
      }
      return Response.redirect(target, 302);
    }
    // Keep one canonical markup source. The English route is selected by the
    // host page script, while the asset request itself still resolves to the
    // same index document.
    if (url.pathname === "/en" || url.pathname === "/en/") {
      return env.ASSETS.fetch(new Request(new URL("/index.html", url), req));
    }
    return env.ASSETS.fetch(req);
  },
};
