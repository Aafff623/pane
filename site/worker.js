// pane.threetwoa.live. The site is a static Next.js export in out/; wrangler.jsonc
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
//   /update/latest.json the in-app updater's first endpoint. The manifest is
//                       a few hundred bytes, but the installer it points at
//                       is 21 MB — and GitHub release assets reach Chinese
//                       networks at tens of KB/s (measured ~40 KB/s against
//                       ~315 KB/s from Gitee, 2026-10-14). For CN clients
//                       this route rewrites the Windows URL to the Gitee
//                       mirror, leaving every other field and platform
//                       untouched; everyone else gets the manifest as-is.
//                       The mirror is probed before it is offered, so a
//                       missing or broken Gitee copy costs speed, never
//                       availability — and the app's second endpoint is
//                       GitHub's own manifest, so this route may fail freely.
//
// Everything else is the static export in out/, via env.ASSETS.fetch.
// (The workers.dev trial address is off: wrangler disables it once a
// custom-domain route is declared.)
//
// Language: / is English, /zh is Chinese; legacy /en/* redirects to the root
// script applies English copy. ?lang=en remains a supported English entry.
//
// No analytics beacon: Magpie injects Cloudflare Web Analytics here, but that
// needs a token tied to a zone we don't have yet. Add it when a real domain
// lands — one edge function, no markup change.

const REPO = "Aafff623/pane";
const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;
const INSTALLER = /-setup\.exe$/;
// The Gitee mirror of the same release. The release workflow uploads the
// Windows installer there (same bytes, same minisign signature); the updater
// verifies the signature client-side, so this host is never trusted.
const GITEE_REPO = "fan-tengda/pane";
const MANIFEST_URL = `https://github.com/${REPO}/releases/latest/download/latest.json`;
const GITEE_ASSET = (version) =>
  `https://gitee.com/${GITEE_REPO}/releases/download/v${version}/Pane-Aafff623_${version}_x64-setup.exe`;

// latestRelease asks GitHub for the newest release. The cf.cacheTtl keeps the
// API call off the request path for repeat visitors (GitHub allows 60
// anonymous calls an hour per IP). Both /download/windows and /api/latest
// read it, so the two surfaces never disagree about "latest".
async function latestRelease() {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json", "User-Agent": "pane-site" },
    signal: AbortSignal.timeout(4000),
    cf: { cacheTtl: 600, cacheEverything: true },
  });
  if (!res.ok) return null;
  try {
    return await res.json();
  } catch {
    return null;
  }
}

async function latestWindowsInstaller() {
  const release = await latestRelease();
  const asset = (release?.assets || []).find((item) => INSTALLER.test(item.name || ""));
  return asset?.browser_download_url ?? null;
}

// /api/latest — the release facts the landing page needs for platform
// routing: version, tag, release page and the asset list. The page picks the
// asset that matches the visitor (os + arch); unknown shapes just leave the
// static download links in place.
async function latestJson() {
  const release = await latestRelease();
  if (!release || typeof release.tag_name !== "string") {
    return new Response(JSON.stringify({ error: "unavailable" }), {
      status: 502,
      headers: { "content-type": "application/json; charset=utf-8", "cache-control": "no-store" },
    });
  }
  const assets = (release.assets || [])
    .filter((item) => item?.name && item?.browser_download_url)
    .map((item) => ({ name: item.name, url: item.browser_download_url, size: item.size ?? null }));
  return new Response(
    JSON.stringify({
      version: release.tag_name.replace(/^v/, ""),
      tag: release.tag_name,
      url: release.html_url ?? null,
      publishedAt: release.published_at ?? null,
      assets,
    }),
    {
      status: 200,
      headers: { "content-type": "application/json; charset=utf-8", "cache-control": "public, max-age=300" },
    },
  );
}

// updateManifest serves the updater manifest, pointing CN clients at the
// Gitee-mirrored installer. Requests for /update/latest.json are made by
// pane.exe (not a browser), so request.cf is always populated; the header
// fallback exists for local dev and tests.
async function updateManifest(req) {
  const upstream = await fetch(MANIFEST_URL, { signal: AbortSignal.timeout(5000), cf: { cacheTtl: 300, cacheEverything: true } });
  if (!upstream.ok) return new Response("updater manifest unavailable", { status: 502 });
  const body = await upstream.text();
  const source = new URL(req.url).searchParams.get("source");
  const country = req.cf?.country ?? req.headers.get("CF-IPCountry") ?? "";
  const wantGitee = source === "gitee" || (source !== "github" && country === "CN");
  const plain = () => new Response(body, { headers: manifestHeaders("github") });
  if (!wantGitee) return plain();
  let doc;
  try {
    doc = JSON.parse(body);
  } catch {
    return plain();
  }
  const version = doc?.version;
  const win = doc?.platforms?.["windows-x86_64"];
  if (!version || typeof win?.url !== "string") return plain();
  const mirror = GITEE_ASSET(version);
  if (!(await giteeMirrorReady(mirror))) return plain();
  win.url = mirror;
  return new Response(JSON.stringify(doc), { headers: manifestHeaders("gitee") });
}

// giteeMirrorReady probes the mirror asset before it is offered. Gitee
// answers a HEAD with a redirect to its object storage and 404 for a missing
// attachment; anything unexpected counts as "not ready" so the caller keeps
// the GitHub URL. The cf.cacheTtl keeps the probe off repeat requests.
async function giteeMirrorReady(url) {
  try {
    const res = await fetch(url, {
      method: "HEAD",
      redirect: "follow",
      signal: AbortSignal.timeout(2500),
      cf: { cacheTtl: 600, cacheEverything: true },
    });
    return res.ok;
  } catch {
    return false;
  }
}

function manifestHeaders(via) {
  return {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "public, max-age=300",
    // Debugging aid only — the updater ignores unknown headers.
    "x-pane-manifest-via": via,
  };
}

function skinCorsHeaders() {
  return {
    "access-control-allow-origin": "*",
    "access-control-allow-methods": "GET, HEAD, OPTIONS",
    "access-control-max-age": "86400",
  };
}

const CSP_REPORT_ONLY = [
  "default-src 'self'",
  "script-src 'self' 'unsafe-inline'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: https:",
  "font-src 'self'",
  "frame-src 'self'",
  "connect-src 'self' https://api.github.com",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
].join("; ");

function withSecurityHeaders(response, pathname) {
  const headers = new Headers(response.headers);
  headers.set("x-content-type-options", "nosniff");
  headers.set("referrer-policy", "strict-origin-when-cross-origin");
  headers.set("permissions-policy", "camera=(), microphone=(), geolocation=()");
  // The demo runs in a same-origin iframe on the landing page, so its own
  // document must allow same-origin framing while every other route keeps
  // refusing to be framed at all.
  const demo = pathname.startsWith("/pane-demo/");
  headers.set("x-frame-options", demo ? "SAMEORIGIN" : "DENY");
  headers.set(
    "content-security-policy-report-only",
    demo ? CSP_REPORT_ONLY.replace("frame-ancestors 'none'", "frame-ancestors 'self'") : CSP_REPORT_ONLY,
  );
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}

async function notFound(env, req) {
  const assetUrl = new URL(req.url);
  assetUrl.pathname = "/404.html";
  const asset = await env.ASSETS.fetch(new Request(assetUrl, req));
  const headers = new Headers(asset.headers);
  headers.delete("location");
  return new Response(asset.body, { status: 404, statusText: "Not Found", headers });
}

async function handle(req, env) {
    const url = new URL(req.url);
    if (url.pathname === "/api/latest") {
      return latestJson();
    }
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
    if (url.pathname === "/update/latest.json") {
      return updateManifest(req);
    }
    // The in-app workshop fetches the index through Rust, but the static
    // files still carry no CORS header — a browser/demo or a future web
    // client must be able to read the index and the packs. GET/HEAD only;
    // OPTIONS is answered directly so preflights never touch the asset store.
    if (url.pathname.startsWith("/skins/")) {
      if (req.method === "OPTIONS") {
        return new Response(null, { status: 204, headers: skinCorsHeaders() });
      }
      const res = await env.ASSETS.fetch(req);
      const headers = new Headers(res.headers);
      headers.set("access-control-allow-origin", "*");
      headers.set("access-control-allow-methods", "GET, HEAD, OPTIONS");
      headers.set("access-control-max-age", "86400");
      return new Response(res.body, { status: res.status, statusText: res.statusText, headers });
    }
    // English now lives at the root; keep the old /en/* URLs working.
    if (url.pathname === "/en" || url.pathname.startsWith("/en/")) {
      const stripped = url.pathname.replace(/^\/en(?=\/|$)/, "") || "/";
      return Response.redirect(new URL(stripped, url).toString(), 301);
    }
    const asset = await env.ASSETS.fetch(req);
    if (asset.status === 404) return notFound(env, req);
    return asset;
}

export default {
  async fetch(req, env) {
    return withSecurityHeaders(await handle(req, env), new URL(req.url).pathname);
  },
};
