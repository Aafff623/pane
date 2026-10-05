// pane.<account-subdomain>.workers.dev. The site is static files in public/;
// this worker adds language negotiation on top of them.
//
//   /        the home page in Chinese (the source language of
//            public/index.html). A browser that prefers English is sent to
//            /en/ until a language is picked on the page (the lang cookie).
//   /en/     the same page in English: public/index.html rewritten at the
//            edge with the dictionary in i18n.js.
//
// Everything else is the static site in public/, via env.ASSETS.fetch —
// wrangler.jsonc sets run_worker_first, so even assets pass through here.
//
// TODO (Phase 2): /api/latest — proxy GitHub's releases/latest API for
// Aafff623/pane into {version, url, published, assets:{name:{url,size,sha256}}}
// (the release workflow already publishes latest.json + sha256 for the app's
// updater; mirror that shape). Phase 1's download buttons link to
// github.com/Aafff623/pane/releases/latest directly, which never goes stale.
//
// No analytics beacon: Magpie injects Cloudflare Web Analytics here, but that
// needs a token tied to a zone we don't have yet. Add it when a real domain
// lands — one edge function, no markup change.

import { LANGS } from "./i18n.js";

const SOURCE = "zh"; // the language public/index.html is written in

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method !== "GET" && req.method !== "HEAD") return env.ASSETS.fetch(req);

    // /en or /en/index.html → /en/; a language page is the source page
    // fetched afresh (its ETag would also stand for an older dictionary)
    // and rewritten into that language.
    const home = url.pathname.match(/^\/([a-z]{2})(\/(index\.html)?)?$/);
    if (home && LANGS[home[1]]) {
      if (!home[2]) return Response.redirect(new URL(`/${home[1]}/`, url).toString(), 301);
      const res = await env.ASSETS.fetch(new Request(new URL("/", url), { method: req.method }));
      return translate(res, home[1]);
    }

    if (url.pathname === "/") {
      const lang = preferred(req);
      const vary = { Vary: "Accept-Language, Cookie", "Cache-Control": "no-cache" };
      if (lang !== SOURCE) return new Response(null, { status: 302, headers: { Location: `/${lang}/`, ...vary } });
      const res = await env.ASSETS.fetch(req);
      const out = new Response(res.body, res);
      out.headers.append("Vary", "Accept-Language, Cookie");
      return out;
    }

    return env.ASSETS.fetch(req);
  },
};

// preferred is the home page's language for this browser: the one picked on
// the page (the lang cookie), else the first of its Accept-Language that the
// site has, else the source language.
export function preferred(req) {
  const picked = (req.headers.get("Cookie") || "").match(/(?:^|;\s*)lang=([a-z]{2})/);
  if (picked) return LANGS[picked[1]] ? picked[1] : SOURCE;
  const wants = (req.headers.get("Accept-Language") || "")
    .split(",")
    .map((p, i) => {
      const [tag, ...rest] = p.trim().toLowerCase().split(";");
      const q = rest.map((x) => x.trim()).find((x) => x.startsWith("q="));
      return { lang: tag.split("-")[0], q: q ? parseFloat(q.slice(2)) || 0 : 1, i };
    })
    .filter((w) => w.lang && w.q > 0)
    .sort((a, b) => b.q - a.q || a.i - b.i);
  for (const w of wants) {
    if (w.lang === SOURCE) return SOURCE;
    if (LANGS[w.lang]) return w.lang;
  }
  return SOURCE;
}

// translate puts a language's strings into the Chinese home page: the inner
// HTML of each data-i18n element, the attributes data-i18n-attr names,
// <html lang>, and the brand link made the language's own. The rewritten
// response's ETag is dropped — the content no longer matches the asset's.
function translate(res, lang) {
  const { dict, html } = LANGS[lang];
  const out = new Response(res.body, res);
  out.headers.delete("ETag");
  return new HTMLRewriter()
    .on("html", { element: (el) => el.setAttribute("lang", html) })
    .on("[data-i18n]", {
      element: (el) => {
        const v = dict[el.getAttribute("data-i18n")];
        if (v != null) el.setInnerContent(v, { html: true });
      },
    })
    .on("[data-i18n-attr]", {
      element: (el) => {
        for (const pair of el.getAttribute("data-i18n-attr").split(",")) {
          const [attr, key] = pair.split(":");
          if (dict[key] != null) el.setAttribute(attr, dict[key]);
        }
      },
    })
    .on("a.brand", { element: (el) => el.setAttribute("href", `/${lang}/`) })
    .transform(out);
}
