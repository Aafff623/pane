import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

// Evaluate the real pure functions without starting Tauri or the DOM entrypoint.
const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
const names = ['cardMetricRows', 'isCoreQuotaMetric', 'isDerivedCeilingDetail', 'isSnapshotMaxed', 'extractOverviewQuota', 'healthDotTitle', 'scheduleBackoffRetries', 'metricNotStarted'];
const functions = ast.statements.filter((node) => ts.isFunctionDeclaration(node) && names.includes(node.name?.text));
assert.equal(functions.length, names.length);
const context = vm.createContext({
  TREND_KEY: '__trend__', MAXED_PCT: 99.5, METRIC_POOLS: {},
  OVERVIEW_PRIMARY_LABEL: {}, OVERVIEW_KEEP_BINDING: new Set(), overviewTab: 'week',
  providerFamily: (id) => id.split('@')[0],
  is5hPeriod: () => false, is5hLabel: () => false,
  metricWindow: () => 'other', nearestResetSeconds: () => 0,
  lastSnapshots: [], supportsExtraAccounts: () => true, isParallelAccountFamily: () => false,
  cardHealthDot: () => 'green', t: (key) => key,
});
vm.runInContext(ts.transpile(functions.map((node) => node.getText(ast)).join('\n')), context);
const plain = (value) => JSON.parse(JSON.stringify(value));
const layout = (extra = {}) => ({ metricOrder: [], hidden: [], onDemand: [], ...extra });
const metrics = ['Balance', 'Cash', 'Vouchers'].map((label) => ({ label }));

test('obsolete labels and all-on-demand rows retain one visible reading without mutating layout', () => {
  const saved = layout({ metricOrder: ['Credits', ...metrics.map((m) => m.label)], onDemand: metrics.map((m) => m.label) });
  const before = structuredClone(saved);
  assert.deepEqual(plain(context.cardMetricRows(saved, metrics)), { always: ['Balance'], onDemand: ['Cash', 'Vouchers'] });
  assert.deepEqual(saved, before);
});

test('empty-layout fallback respects explicitly hidden metrics', () => {
  assert.deepEqual(plain(context.cardMetricRows(layout({ hidden: ['Cash'] }), metrics)), {
    always: ['Balance', 'Vouchers'], onDemand: [],
  });
  assert.deepEqual(plain(context.cardMetricRows(layout({ hidden: metrics.map((m) => m.label) }), metrics)), {
    always: [], onDemand: [],
  });
});

test('historical derived ceiling stays visible in overview without exhausting live provider', () => {
  const derived = { kind: 'progress', label: 'app', used_percent: 100, detail: '2400 of 2400 calls · last completed day 2026-10-14 · ceiling derived from 100 calls/hour × 24', period_ms: null, resets_at: null };
  const snap = { id: 'giphy', status: 'ok', plan: null, metrics: [derived] };
  assert.equal(context.isSnapshotMaxed(snap), false);
  const quota = context.extractOverviewQuota(snap);
  assert.equal(quota.usedPercent, 100);
  assert.equal(quota.derivedCeiling, true);
  assert.equal(quota.isMaxed, false);
  assert.equal(quota.status, 'ok');
  assert.equal(context.isSnapshotMaxed({ ...snap, metrics: [{ ...derived, detail: 'live usage' }] }), true);
  const searchApp = { ...derived, label: 'hermes-search', used_percent: 70 };
  assert.equal(context.isCoreQuotaMetric(searchApp), true);
  assert.equal(context.extractOverviewQuota({ ...snap, metrics: [{ ...derived, used_percent: 10 }, searchApp] }).usedPercent, 70);
  assert.equal(context.isCoreQuotaMetric({ ...searchApp, detail: 'ordinary web searches quota' }), false);
});

test('healthy peak-hour account does not inherit sibling setup tooltip', () => {
  context.lastSnapshots = [{ id: 'stepfun-plan@a', status: 'ok' }, { id: 'stepfun-plan@b', status: 'needs_setup' }];
  assert.equal(context.healthDotTitle('yellow', 'stepfun-plan@a'), 'customize.acctDotYellow');
  assert.equal(context.healthDotTitle('yellow', 'stepfun-plan@b'), 'customize.acctDotSetup');
});

test('needs_setup cancels a previously queued failure retry', () => {
  const cleared = [];
  context.clearBackoffRetry = (id) => cleared.push(id);
  context.isCardDisabled = () => false;
  context.scheduleBackoffRetries([{ id: 'stepfun-plan', status: 'needs_setup', stale: false }]);
  assert.deepEqual(cleared, ['stepfun-plan']);
});

test('fixed StepFun hints localize without changing English API messages', () => {
  const i18nSource = readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
  const exports = {};
  vm.runInNewContext(ts.transpile(i18nSource, { module: ts.ModuleKind.CommonJS }), { exports });
  const rust = readFileSync(new URL('../src-tauri/src/providers/stepfun_plan.rs', import.meta.url), 'utf8');
  const hints = [...rust.slice(rust.indexOf('fn setup_hint('), rust.indexOf('/// The stored Oasis-Webid')).matchAll(/"((?:[^"\\]|\\.)*)"\.into\(\)/g)].map((m) => JSON.parse(`"${m[1]}"`));
  assert.equal(hints.length, 4);
  for (const locale of ['zh', 'ru']) {
    exports.setActiveLocale(locale);
    for (const hint of hints) {
      assert.notEqual(exports.displayCardHint(hint), hint, `${locale} must localize ${hint.slice(0, 30)}`);
    }
    const day = exports.displayMetricDetail('23 calls · last completed day 2026-10-14');
    assert.match(day, /23/);
    assert.match(day, /2026-10-14/);
    assert.doesNotMatch(day, /last completed day/);
    assert.equal(exports.displayCardHint('unrecognized server message'), 'unrecognized server message');
  }
});
