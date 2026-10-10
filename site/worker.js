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
// Language: / is Chinese; /en and /en/ serve the same document and the host
// script applies English copy. ?lang=en remains a supported English entry.
//
// No analytics beacon: Magpie injects Cloudflare Web Analytics here, but that
// needs a token tied to a zone we don't have yet. Add it when a real domain
// lands — one edge function, no markup change.

import { en } from "./i18n.js";

const REPO = "Aafff623/pane";
const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;
const INSTALLER = /-setup\.exe$/;

// Serve English metadata in the initial HTML so search and sharing clients
// receive the right title and canonical URL without executing site.js.
async function englishMetadata(response) {
  if (!response.ok || !response.headers.get("content-type")?.includes("text/html")) return response;
  const html = (await response.text())
    .replace('<html lang="zh-CN"', '<html lang="en"')
    .replace(/<title>[^<]*<\/title>/, `<title>${en.metaTitle}</title>`)
    .replace(/(<meta (?:name="description"|property="og:description"|name="twitter:description") content=")[^"]*(")/g, `$1${en.metaDescription}$2`)
    .replace(/(<meta (?:property="og:title"|name="twitter:title") content=")[^"]*(")/g, `$1${en.metaTitle}$2`)
    .replace('<link rel="canonical" href="https://pane.threetwoa.live/">', '<link rel="canonical" href="https://pane.threetwoa.live/en/">')
    .replace('<meta property="og:url" content="https://pane.threetwoa.live/">', '<meta property="og:url" content="https://pane.threetwoa.live/en/">')
    .replace('<meta property="og:locale" content="zh_CN">', '<meta property="og:locale" content="en_US">');
  const headers = new Headers(response.headers);
  for (const name of ["content-length", "content-encoding", "etag"]) headers.delete(name);
  headers.set("content-language", "en");
  return new Response(html, { status: response.status, statusText: response.statusText, headers });
}

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
    if (url.pathname === "/en" || url.pathname === "/en/" || (url.pathname === "/" && url.searchParams.get("lang") === "en")) {
      const assetUrl = new URL(url);
      // Request the root document: /index.html is canonicalized to / by the
      // asset server, which would redirect the browser out of English mode.
      assetUrl.pathname = "/";
      return englishMetadata(await env.ASSETS.fetch(new Request(assetUrl, req)));
    }
    return env.ASSETS.fetch(req);
  },
};
