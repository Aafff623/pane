var __defProp = Object.defineProperty;
var __name = (target, value) => __defProp(target, "name", { value, configurable: true });

// i18n.js
var en = {
  "meta.desc": "Pane is a Windows tray panel that keeps the quotas, balances and spend of 41 AI coding tools and model vendors in one place. Keys stay on your machine; queries go straight to each vendor's own API.",
  "meta.tagline": "Every AI tool's quota and spend, at a glance.",
  "nav.how": "How it works",
  "nav.providers": "Providers",
  "nav.shots": "Screenshots",
  "nav.api": "Local API",
  "nav.releases": "Releases",
  "nav.language": "Language",
  "nav.menu": "Menu",
  "nav.download": "Download",
  "hero.h1": "Every AI tool's quota and spend, <span>at a glance.</span>",
  "hero.lede": "Pane is a local-first panel that lives in the Windows tray, gathering the quotas, balances and subscription states of the AI coding tools and model vendors you use \u2014 Claude, Codex, Cursor, Kimi and 41 in all. Keys stay on your machine, queries go straight to each vendor's own API, and your data never leaves your computer.",
  "hero.dl": "Download the latest release",
  "hero.gh": "GitHub repository",
  "hero.free": "Free",
  "hero.oss": "Open source (MIT)",
  "hero.os": "Windows-first; macOS and Linux builds ship with each release",
  "alt.overview": "The Pane panel: quota progress and reset times for each AI tool",
  "alt.spend": "Pane's spend panel: tokens and cost grouped by tool and model",
  "alt.heatmap": "Pane's 365-day daily spend heatmap",
  "prov.eyebrow": "Providers",
  "prov.h2": "41 vendors, one wall.",
  "prov.sub": "Model vendors, relay services and local servers, plus custom balance queries \u2014 from Claude, Codex and Cursor to DeepSeek, Kimi and SiliconFlow, with the list growing every release.",
  "how.eyebrow": "How it works",
  "how.h2": "Three steps, all on your machine.",
  "how.sub": "No accounts, no cloud sync. Pane talks only to your computer and to each vendor's own API.",
  "how.s1.t": "Add credentials",
  "how.s1.p": "Add API keys or OAuth logins to Pane. Keys are written only to Pane's local data directory, and an optional master-password vault locks them further.",
  "how.s2.t": "Query locally",
  "how.s2.p": "Pane polls each vendor's own API for quotas and balances, and scans local CLI logs for spend \u2014 never through a third-party server.",
  "how.s3.t": "One panel",
  "how.s3.p": "The tray icon projects the tightest quota in real time; open the panel for per-vendor progress, reset countdowns and the spend heatmap.",
  "det.eyebrow": "Details",
  "det.h2": "Local-first, taken seriously.",
  "det.local.t": "Keys never leave the machine",
  "det.local.p": "Tokens go only to their own vendor's API. The local endpoint binds to loopback, checks the Host header and has no CORS.",
  "det.vault.t": "Master-password vault",
  "det.vault.p": "An optional encrypted vault under a master password adds another lock over locally stored credentials, with notes per key.",
  "det.spend.t": "Spend stats and heatmap",
  "det.spend.p": "Tokens and cost grouped by tool and by model, with a daily heatmap reaching back 365 days.",
  "det.tray.t": "Lives in the tray",
  "det.tray.p": "Rests quietly in the Windows tray, with optional toasts when a quota runs low. It does not nag.",
  "det.update.t": "Signed auto-updates",
  "det.update.p": "New versions are downloaded from GitHub Releases, signature-checked and installed \u2014 no website visit needed.",
  "det.api.t": "Local JSON API",
  "det.api.p": "A read-only endpoint hands the panel's data to your own scripts and automation, below.",
  "shot.eyebrow": "Screenshots",
  "shot.h2": "The interface, as it is.",
  "shot.sub": "One shot per theme with fixed file names \u2014 replace the files of the same name under site/public/img/ to update.",
  "shot.spend.cap": "Spend by tool and by model",
  "shot.heat.cap": "365 days of daily spend",
  "shot.note": "These are placeholders: swap overview / spend / heatmap -dark.png and -light.png in place \u2014 no code change needed.",
  "api.eyebrow": "Local API",
  "api.h2": "The numbers, for your scripts.",
  "api.sub": "Pane exposes a read-only JSON endpoint on your machine that only answers loopback requests. Everything the panel shows, your scripts can read.",
  "api.c1": "# the newest snapshot of every provider",
  "api.c2": "# a single provider",
  "api.note": "The endpoint binds to 127.0.0.1 only, checks the Host header and sends no CORS headers; keys never appear in any response.",
  "copy": "Copy",
  "fin.h2": "Fewer tabs to check. One glance at the tray.",
  "fin.sub": "Free, open source, local-first. Windows-first; macOS and Linux builds ship with each release.",
  "fin.dl": "Download the latest release",
  "fin.gh": "GitHub repository",
  "foot.tag": "Pane \u2014 the local-first AI quota panel",
  "foot.src": "Source",
  "foot.rel": "Releases",
  "foot.issues": "Issues"
};
var LANGS = {
  en: { dict: en, html: "en" }
};

// worker.js
var SOURCE = "zh";
var worker_default = {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method !== "GET" && req.method !== "HEAD") return env.ASSETS.fetch(req);
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
  }
};
function preferred(req) {
  const picked = (req.headers.get("Cookie") || "").match(/(?:^|;\s*)lang=([a-z]{2})/);
  if (picked) return LANGS[picked[1]] ? picked[1] : SOURCE;
  const wants = (req.headers.get("Accept-Language") || "").split(",").map((p, i) => {
    const [tag, ...rest] = p.trim().toLowerCase().split(";");
    const q = rest.map((x) => x.trim()).find((x) => x.startsWith("q="));
    return { lang: tag.split("-")[0], q: q ? parseFloat(q.slice(2)) || 0 : 1, i };
  }).filter((w) => w.lang && w.q > 0).sort((a, b) => b.q - a.q || a.i - b.i);
  for (const w of wants) {
    if (w.lang === SOURCE) return SOURCE;
    if (LANGS[w.lang]) return w.lang;
  }
  return SOURCE;
}
__name(preferred, "preferred");
function translate(res, lang) {
  const { dict, html } = LANGS[lang];
  const out = new Response(res.body, res);
  out.headers.delete("ETag");
  return new HTMLRewriter().on("html", { element: /* @__PURE__ */ __name((el) => el.setAttribute("lang", html), "element") }).on("[data-i18n]", {
    element: /* @__PURE__ */ __name((el) => {
      const v = dict[el.getAttribute("data-i18n")];
      if (v != null) el.setInnerContent(v, { html: true });
    }, "element")
  }).on("[data-i18n-attr]", {
    element: /* @__PURE__ */ __name((el) => {
      for (const pair of el.getAttribute("data-i18n-attr").split(",")) {
        const [attr, key] = pair.split(":");
        if (dict[key] != null) el.setAttribute(attr, dict[key]);
      }
    }, "element")
  }).on("a.brand", { element: /* @__PURE__ */ __name((el) => el.setAttribute("href", `/${lang}/`), "element") }).transform(out);
}
__name(translate, "translate");

// ../../../nodejs/node_cache/_npx/32026684e21afda6/node_modules/wrangler/templates/middleware/middleware-ensure-req-body-drained.ts
var drainBody = /* @__PURE__ */ __name(async (request, env, _ctx, middlewareCtx) => {
  try {
    return await middlewareCtx.next(request, env);
  } finally {
    try {
      if (request.body !== null && !request.bodyUsed) {
        const reader = request.body.getReader();
        while (!(await reader.read()).done) {
        }
      }
    } catch (e) {
      console.error("Failed to drain the unused request body.", e);
    }
  }
}, "drainBody");
var middleware_ensure_req_body_drained_default = drainBody;

// ../../../nodejs/node_cache/_npx/32026684e21afda6/node_modules/wrangler/templates/middleware/middleware-miniflare3-json-error.ts
function reduceError(e) {
  return {
    name: e?.name,
    message: e?.message ?? String(e),
    stack: e?.stack,
    cause: e?.cause === void 0 ? void 0 : reduceError(e.cause)
  };
}
__name(reduceError, "reduceError");
var jsonError = /* @__PURE__ */ __name(async (request, env, _ctx, middlewareCtx) => {
  try {
    return await middlewareCtx.next(request, env);
  } catch (e) {
    const error = reduceError(e);
    const body = JSON.stringify(error);
    const headers = {
      "Content-Type": "application/json",
      "MF-Experimental-Error-Stack": "true"
    };
    const encoded = encodeURIComponent(body);
    if (encoded.length <= 8192) {
      headers["MF-Experimental-Error-Stack-Payload"] = encoded;
    }
    return new Response(body, { status: 500, headers });
  }
}, "jsonError");
var middleware_miniflare3_json_error_default = jsonError;

// .wrangler/tmp/bundle-c3c0NR/middleware-insertion-facade.js
var __INTERNAL_WRANGLER_MIDDLEWARE__ = [
  middleware_ensure_req_body_drained_default,
  middleware_miniflare3_json_error_default
];
var middleware_insertion_facade_default = worker_default;

// ../../../nodejs/node_cache/_npx/32026684e21afda6/node_modules/wrangler/templates/middleware/common.ts
var __facade_middleware__ = [];
function __facade_register__(...args) {
  __facade_middleware__.push(...args.flat());
}
__name(__facade_register__, "__facade_register__");
function __facade_invokeChain__(request, env, ctx, dispatch, middlewareChain) {
  const [head, ...tail] = middlewareChain;
  const middlewareCtx = {
    dispatch,
    next(newRequest, newEnv) {
      return __facade_invokeChain__(newRequest, newEnv, ctx, dispatch, tail);
    }
  };
  return head(request, env, ctx, middlewareCtx);
}
__name(__facade_invokeChain__, "__facade_invokeChain__");
function __facade_invoke__(request, env, ctx, dispatch, finalMiddleware) {
  return __facade_invokeChain__(request, env, ctx, dispatch, [
    ...__facade_middleware__,
    finalMiddleware
  ]);
}
__name(__facade_invoke__, "__facade_invoke__");

// .wrangler/tmp/bundle-c3c0NR/middleware-loader.entry.ts
var __Facade_ScheduledController__ = class ___Facade_ScheduledController__ {
  constructor(scheduledTime, cron, noRetry) {
    this.scheduledTime = scheduledTime;
    this.cron = cron;
    this.#noRetry = noRetry;
  }
  scheduledTime;
  cron;
  static {
    __name(this, "__Facade_ScheduledController__");
  }
  #noRetry;
  noRetry() {
    if (!(this instanceof ___Facade_ScheduledController__)) {
      throw new TypeError("Illegal invocation");
    }
    this.#noRetry();
  }
};
function wrapExportedHandler(worker) {
  if (__INTERNAL_WRANGLER_MIDDLEWARE__ === void 0 || __INTERNAL_WRANGLER_MIDDLEWARE__.length === 0) {
    return worker;
  }
  for (const middleware of __INTERNAL_WRANGLER_MIDDLEWARE__) {
    __facade_register__(middleware);
  }
  const fetchDispatcher = /* @__PURE__ */ __name(function(request, env, ctx) {
    if (worker.fetch === void 0) {
      throw new Error("Handler does not export a fetch() function.");
    }
    return worker.fetch(request, env, ctx);
  }, "fetchDispatcher");
  return {
    ...worker,
    fetch(request, env, ctx) {
      const dispatcher = /* @__PURE__ */ __name(function(type, init) {
        if (type === "scheduled" && worker.scheduled !== void 0) {
          const controller = new __Facade_ScheduledController__(
            Date.now(),
            init.cron ?? "",
            () => {
            }
          );
          return worker.scheduled(controller, env, ctx);
        }
      }, "dispatcher");
      return __facade_invoke__(request, env, ctx, dispatcher, fetchDispatcher);
    }
  };
}
__name(wrapExportedHandler, "wrapExportedHandler");
function wrapWorkerEntrypoint(klass) {
  if (__INTERNAL_WRANGLER_MIDDLEWARE__ === void 0 || __INTERNAL_WRANGLER_MIDDLEWARE__.length === 0) {
    return klass;
  }
  for (const middleware of __INTERNAL_WRANGLER_MIDDLEWARE__) {
    __facade_register__(middleware);
  }
  return class extends klass {
    #fetchDispatcher = /* @__PURE__ */ __name((request, env, ctx) => {
      this.env = env;
      this.ctx = ctx;
      if (super.fetch === void 0) {
        throw new Error("Entrypoint class does not define a fetch() function.");
      }
      return super.fetch(request);
    }, "#fetchDispatcher");
    #dispatcher = /* @__PURE__ */ __name((type, init) => {
      if (type === "scheduled" && super.scheduled !== void 0) {
        const controller = new __Facade_ScheduledController__(
          Date.now(),
          init.cron ?? "",
          () => {
          }
        );
        return super.scheduled(controller);
      }
    }, "#dispatcher");
    fetch(request) {
      return __facade_invoke__(
        request,
        this.env,
        this.ctx,
        this.#dispatcher,
        this.#fetchDispatcher
      );
    }
  };
}
__name(wrapWorkerEntrypoint, "wrapWorkerEntrypoint");
var WRAPPED_ENTRY;
if (typeof middleware_insertion_facade_default === "object") {
  WRAPPED_ENTRY = wrapExportedHandler(middleware_insertion_facade_default);
} else if (typeof middleware_insertion_facade_default === "function") {
  WRAPPED_ENTRY = wrapWorkerEntrypoint(middleware_insertion_facade_default);
}
var middleware_loader_entry_default = WRAPPED_ENTRY;
export {
  __INTERNAL_WRANGLER_MIDDLEWARE__,
  middleware_loader_entry_default as default,
  preferred
};
//# sourceMappingURL=worker.js.map
