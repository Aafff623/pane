// Workshop preview, startup sequence and first-paint state regressions for
// the comic-pixel skin. Zero dependencies: the real sources are compiled and
// run in a vm context with DOM/GSAP stubs, the same pattern as the other
// main.ts tests in this folder.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const mainSource = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const mainAst = ts.createSourceFile("main.ts", mainSource, ts.ScriptTarget.Latest, true);

function mainFns(...names) {
  const code = mainAst.statements
    .filter((node) => ts.isFunctionDeclaration(node) && names.includes(node.name?.text))
    .map((node) => node.getText(mainAst))
    .join("\n");
  return ts.transpile(code, { target: ts.ScriptTarget.ES2020 });
}

const SAMPLE_FNS = [
  "renderSpendSkinSampleBody",
  "renderQuotaSkinSampleBody",
  "renderSettingsSkinSampleBody",
  "renderSkinSample",
];

function sampleContext() {
  const context = vm.createContext({
    t: (key) => key,
    escapeHtml: (value) => String(value),
    uiIcon: () => '<svg class="ui-icon"></svg>',
    renderMetric: (metric) =>
      `<div class="metric" data-used="${metric.used_percent}"><div class="bar"><div class="fill${metric.used_percent >= 90 ? " low" : ""}"></div></div><span class="left-val">${metric.used_percent}%</span></div>`,
    TAU: Math.PI * 2,
    sectorPath: () => "M0 0",
    spendColor: () => "#123456",
    skinPreviewPage: "quota",
    Date,
    Math,
  });
  vm.runInContext(mainFns(...SAMPLE_FNS), context);
  return context;
}

test("workshop samples render quota, spend and settings for any pack under its own data-skin", () => {
  const ctx = sampleContext();

  const quota = ctx.renderSkinSample("light", "comic-pixel");
  assert.match(quota, /class="skin-sample" data-skin="comic-pixel"/);
  assert.match(quota, /card-account-tabs/);
  assert.match(quota, /card-account-tab/);
  // The low-quota warning state comes from a real 93% reading, not a class.
  assert.match(quota, /data-used="32"/);
  assert.match(quota, /data-used="93"/);
  assert.match(quota, /class="fill low"/);

  vm.runInContext('skinPreviewPage = "settings"', ctx);
  const settings = ctx.renderSkinSample("dark", "comic-pixel");
  assert.match(settings, /data-skin="comic-pixel"/);
  assert.match(settings, /<select /);
  assert.equal((settings.match(/class="toggle"/g) ?? []).length, 2, "both self-drawn switches must be shown");
  assert.equal((settings.match(/type="checkbox"/g) ?? []).length, 2);
  assert.match(settings, /data-theme="dark"/);

  vm.runInContext('skinPreviewPage = "spend"', ctx);
  const spend = ctx.renderSkinSample("light", "comic-pixel");
  assert.match(spend, /total-spend/);
  assert.match(spend, /data-skin="comic-pixel"/);

  // The same renderer serves every pack: the sample carries the requested id
  // and never leaks another skin's scope into it.
  const other = ctx.renderSkinSample("light", "signal");
  const scopes = [...other.matchAll(/class="skin-sample" data-skin="([^"]+)"/g)].map((m) => m[1]);
  assert.deepEqual(scopes, ["signal"]);
  assert.ok(!other.includes('data-skin="comic-pixel"'));

  // Compact gallery thumbnails stay on the spend card.
  const thumb = ctx.renderSkinSample("light", "comic-pixel", true);
  assert.match(thumb, /total-spend/);
  assert.ok(!thumb.includes("card-account-tabs"), "thumbnails are not the quota page");
});

test("workshop samples only use fixed demo data, never live stores", () => {
  const source = mainFns(...SAMPLE_FNS);
  const live = ["lastSnapshots", "accountsCache", "lastSpend", "lastSpendHistory", "lastSpendDaily", "spendHistory", "invoke("];
  const leaks = live.filter((name) => source.includes(name));
  assert.deepEqual(leaks, [], "a sample must not read live usage/account data");
  // The only literal provider rows are in the quota page; they are fixed.
  assert.match(source, /used: 93/);
});

test("low quota sample value really maps to the warning tier", () => {
  const context = vm.createContext({});
  vm.runInContext(mainFns("usageHeatTone"), context);
  assert.equal(context.usageHeatTone(93), "low");
  assert.equal(context.usageHeatTone(32), "");
});

const BOARD_FNS = ["renderEmptyBoard", "renderBoardLoadingState", "renderBoardReadFailedState", "renderBoardEmptyState"];

function boardContext(phase, enabled) {
  const context = vm.createContext({
    t: (key) => key,
    escapeHtml: (value) => String(value),
    uiIcon: () => '<svg class="ui-icon"></svg>',
    boardLoadPhase: phase,
    enabledProviderCount: () => enabled,
  });
  vm.runInContext(mainFns(...BOARD_FNS), context);
  return context;
}

test("pending first paint renders skeletons only, never the add-provider empty state", () => {
  const html = boardContext("pending", 2).renderEmptyBoard();
  assert.equal((html.match(/class="skeleton-card"/g) ?? []).length, 3);
  assert.ok(!html.includes("board-empty"), "loading and empty must be exclusive");
  assert.ok(!html.includes("board.emptyCta"), "no add-provider CTA while loading");
});

test("a failed first read is 'couldn't read', not 'nothing configured'", () => {
  const html = boardContext("failed", 2).renderEmptyBoard();
  assert.match(html, /board-read-failed/);
  assert.match(html, /data-board-retry/);
  assert.ok(!html.includes("board.emptyCta"), "unreadable must not offer 'add a provider'");
});

test("a finished read with enabled providers but no snapshots is also a read failure", () => {
  const html = boardContext("ready", 3).renderEmptyBoard();
  assert.match(html, /board-read-failed/);
  assert.ok(!html.includes("board.emptyCta"));
});

test("a finished read with nothing enabled shows the genuine empty state", () => {
  const html = boardContext("ready", 0).renderEmptyBoard();
  assert.match(html, /class="board-empty"/);
  assert.match(html, /data-board-empty-cta/);
  assert.ok(!html.includes("board-read-failed"));
  // Wiring: the no-snapshot branch must go through renderEmptyBoard.
  assert.match(mainSource, /if \(snaps\.length === 0\) return renderEmptyBoard\(\);/);
});

// ---------------------------------------------------------------------------
// Startup: comic-pixel owns its sequence; everything else keeps the old one.
// ---------------------------------------------------------------------------

const comicSource = readFileSync(new URL("../src/comicStartup.ts", import.meta.url), "utf8");
const comicAst = ts.createSourceFile("comicStartup.ts", comicSource, ts.ScriptTarget.Latest, true);

function comicCode() {
  return ts.transpile(
    comicAst.statements
      .filter((node) => {
        if (ts.isFunctionDeclaration(node)) return ["hasComicStartup", "element", "playComicStartup"].includes(node.name?.text);
        if (ts.isVariableStatement(node)) {
          const text = node.getText(comicAst);
          return text.includes("COMIC_STARTUP_SKINS") || text.startsWith("let stopIntro");
        }
        return false;
      })
      .map((node) => node.getText(comicAst))
      .join("\n"),
    { target: ts.ScriptTarget.ES2020 },
  ).replace(/^export\s+/gm, "");
}

function makeNode(tag) {
  const node = {
    tagName: tag.toUpperCase(),
    className: "",
    textContent: "",
    children: [],
    dataset: {},
    attributes: {},
    listeners: {},
    removed: false,
    type: "",
    setAttribute(name, value) {
      node.attributes[name] = value;
    },
    append(...items) {
      node.children.push(...items);
    },
    addEventListener(type, handler) {
      node.listeners[type] = handler;
    },
    removeEventListener(type) {
      delete node.listeners[type];
    },
    remove() {
      node.removed = true;
    },
    focus() {},
  };
  return node;
}

class FakeHTMLElement {}

function startupContext({ reduced, gsapFails = false }) {
  const documentListeners = {};
  const timers = [];
  const document = {
    documentElement: {
      dataset: {},
      removeAttribute(name) {
        if (name === "data-theme-intro") delete document.documentElement.dataset.themeIntro;
      },
    },
    body: makeNode("body"),
    activeElement: null,
    createElement: (tag) => makeNode(tag),
    addEventListener(type, handler) {
      documentListeners[type] = handler;
    },
    removeEventListener(type) {
      delete documentListeners[type];
    },
  };
  const chain = {
    fromTo() {
      return chain;
    },
    to() {
      return chain;
    },
    kill() {
      chain.killed = true;
    },
  };
  const gsap = {
    timeline() {
      if (gsapFails) throw new Error("ticker unavailable");
      return chain;
    },
  };
  const context = vm.createContext({
    document,
    gsap,
    HTMLElement: FakeHTMLElement,
    setTimeout(handler, delay) {
      timers.push({ handler, delay });
      return timers.length;
    },
    clearTimeout() {},
    Error,
  });
  vm.runInContext(comicCode(), context);
  const labels = { name: "Comic Pixel", tagline: "v1", skip: "Skip", aria: "Intro" };
  return { context, document, documentListeners, timers, chain, labels };
}

test("startup sequence is bound to comic-pixel only", () => {
  const ctx = startupContext({ reduced: true }).context;
  assert.equal(ctx.hasComicStartup("comic-pixel"), true);
  assert.equal(ctx.hasComicStartup("signal"), false);
  assert.equal(ctx.hasComicStartup("ink"), false);
});

test("reduced motion shows the static frame and closes on its own", () => {
  const { context, document, timers, labels } = startupContext({ reduced: true });
  const close = context.playComicStartup(true, labels);
  const overlay = document.body.children[0];
  assert.equal(overlay.dataset.phase, "settled");
  assert.equal(timers[0].delay, 1100);
  assert.equal(document.documentElement.dataset.themeIntro, "comic-pixel");
  close();
  assert.equal(overlay.removed, true);
  assert.equal(document.documentElement.dataset.themeIntro, undefined);
});

test("full motion runs the bounded 3.3 s sequence and Escape closes it", () => {
  const { context, document, documentListeners, timers, labels } = startupContext({ reduced: false });
  context.playComicStartup(false, labels);
  const overlay = document.body.children[0];
  assert.equal(overlay.dataset.phase, "playing");
  assert.equal(timers[0].delay, 3300);
  documentListeners.keydown({
    key: "Escape",
    isComposing: false,
    preventDefault() {},
    stopImmediatePropagation() {},
  });
  assert.equal(overlay.removed, true);
});

test("a dead animation ticker degrades to the static frame instead of throwing", () => {
  const { context, document, timers, labels } = startupContext({ reduced: false, gsapFails: true });
  assert.doesNotThrow(() => context.playComicStartup(false, labels));
  const overlay = document.body.children[0];
  assert.equal(overlay.dataset.phase, "settled");
  assert.equal(timers[0].delay, 3300);
});

test("playThemeStartup routes comic-pixel through the pack sequence and can fall back", async () => {
  const themeSource = readFileSync(new URL("../src/themeStartup.ts", import.meta.url), "utf8");
  assert.match(themeSource, /if \(hasComicStartup\(skin\)\)/);
  assert.match(themeSource, /return undefined;/, "a failed override must let the native splash stay");
});

// ---------------------------------------------------------------------------
// Share PNG: the export already runs after fonts settle and carries the pack.
// ---------------------------------------------------------------------------

test("share export waits for fonts and embeds only the applied pack", () => {
  assert.match(mainSource, /await document\.fonts\.ready;/);
  assert.match(mainSource, /owner\.dataset\.skinPack !== snapshotPack\) continue;/);
  assert.match(mainSource, /data-skin="\$\{root\.dataset\.skin/);
});
