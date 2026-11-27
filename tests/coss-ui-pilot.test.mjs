import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const path = new URL("../src/main.ts", import.meta.url);
const source = ts.createSourceFile(path.pathname, readFileSync(path, "utf8"), ts.ScriptTarget.Latest, true);

function runFunction(name, globals) {
  const fn = source.statements.find((node) => ts.isFunctionDeclaration(node) && node.name?.text === name);
  assert.ok(fn, `Expected ${name} in the app entry point`);
  const code = ts.transpile(fn.getText(source), {
    target: ts.ScriptTarget.ES2020,
    module: ts.ModuleKind.CommonJS,
  });
  const context = vm.createContext(globals);
  vm.runInContext(code, context);
  return context;
}

test("time format shows the latest choice while earlier saves are pending", async () => {
  const config = { timeFormat: "auto" };
  const saves = [];
  let visible;
  const ctx = runFunction("renderTimeFormatControl", {
    IS_PANEL_FORM: true,
    document: { querySelector: () => ({}) },
    config,
    t: (key) => key,
    require: () => ({ renderTimeFormat: (_, props) => { visible = props; } }),
    patchConfig: ({ timeFormat }) => {
      config.timeFormat = timeFormat;
      return new Promise((resolve) => saves.push({ timeFormat, resolve }));
    },
    requestRender() {},
  });
  const flush = () => new Promise((resolve) => setImmediate(resolve));

  ctx.renderTimeFormatControl();
  await flush();
  visible.onChange("24");
  await flush();
  assert.equal(visible.value, "24");

  visible.onChange("auto");
  await flush();
  assert.equal(visible.value, "auto");
  assert.deepEqual(saves.map(({ timeFormat }) => timeFormat), ["24", "auto"]);

  saves[0].resolve();
  await flush();
  assert.equal(visible.value, "auto");
  saves[1].resolve();
  await flush();
  assert.equal(visible.value, "auto");
});

test("help closes on demand without reopening after a pending import", async () => {
  const original = { isConnected: true };
  const replacement = {};
  let mounts = 0;
  let disposals = 0;
  let restoreFocus;
  const ctx = runFunction("openProviderHelp", {
    providerDefinition: () => null,
    getApiKeyLink: () => null,
    PROVIDER_CRED_INFO: {},
    PROVIDER_LINKS: {},
    MECHANISMS: {},
    displayMetricLabel: (metric) => metric,
    t: (key) => key,
    providerVisual: () => null,
    providerDisplayName: (id) => id,
    uiIcon: () => "",
    document: { querySelector: () => replacement },
    dismissProviderHelp: undefined,
    require: () => ({
      showProviderHelp(_, focus, release) {
        mounts++;
        restoreFocus = focus;
        return () => { disposals++; release(); };
      },
    }),
  });
  const flush = () => new Promise((resolve) => setImmediate(resolve));

  ctx.openProviderHelp("codex", original, "[data-provider-help=codex]");
  ctx.dismissProviderHelp();
  await flush();
  assert.equal(mounts, 0);

  ctx.openProviderHelp("codex", original, "[data-provider-help=codex]");
  await flush();
  assert.equal(mounts, 1);
  assert.equal(restoreFocus(), original);
  original.isConnected = false;
  assert.equal(restoreFocus(), replacement);
  ctx.dismissProviderHelp();
  assert.equal(disposals, 1);
  assert.equal(ctx.dismissProviderHelp, undefined);
});

test("provider index keeps Unicode expansions in the non-Latin group", () => {
  const { skeyLetter } = runFunction("skeyLetter", {});
  assert.equal(skeyLetter(" miniMax"), "M");
  assert.equal(skeyLetter("ß工具"), "#");
  assert.equal(skeyLetter("ﬀ工具"), "#");
  assert.equal(skeyLetter("服务商"), "#");
});
