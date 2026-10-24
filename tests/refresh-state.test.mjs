import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import { RefreshState } from '../src/refreshState.ts';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
const refresh = ast.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'refresh');
assert.ok(refresh, 'exercise the production refresh implementation');
const productionRefresh = ts.transpileModule(refresh.getText(ast), {
  compilerOptions: { target: ts.ScriptTarget.ES2022 },
}).outputText;
const nextTurn = () => new Promise(setImmediate);

function refreshWindow(invoke) {
  const state = {
    document: { hidden: true, querySelector: () => ({ textContent: '' }) },
    config: { layout: null, disabled: [], removedProviders: [] },
    lastFetch: 0, STALE_MS: 0,
    lastSnapshots: [], recentlyKeyed: new Map(), firstRunPickerPending: false,
    lastLayoutSnapshot: null, configSaveError: null, customizeOpen: false,
    boardLoadPhase: 'ready',
    lastCreditTrend: null,
    refreshState: new RefreshState(),
    unparkRecentlyKeyed: async () => {}, hideFoldedMoonshot: value => value,
    ensureLayout: () => {}, ensureRtkTargets: () => {}, requestRender: () => {}, scheduleBackoffRetries: () => {},
    requestTraySync: () => {}, writeIdleFooter: () => {}, paintRefreshState: () => {},
    normalizeHistory: value => value, normalizeDaily: value => value,
    normalizeProviderSpend: value => value, providerCatalog: [],
    t: value => value, invoke,
  };
  vm.createContext(state);
  vm.runInContext(productionRefresh, state);
  return state;
}

test('a late credit reading cannot replace a newer successful refresh', async () => {
  const readings = [];
  const state = refreshWindow(command => command === 'fetch_credit_history'
    ? new Promise(resolve => readings.push(resolve))
    : Promise.resolve([]));
  const older = state.refresh(true, true);
  await nextTurn();
  const newer = state.refresh(true, true);
  await nextTurn();
  readings[1]({ provider: [200] });
  await newer;
  readings[0]({ provider: [100] });
  await older;
  assert.equal(state.lastCreditTrend.provider[0], 200);
});

test('a queued manual refresh retains its permission to clear ordinary benches', async () => {
  const requests = [];
  let finishFirst;
  const state = refreshWindow((command, options) => {
    if (command !== 'fetch_usage') return Promise.resolve([]);
    requests.push(options);
    return requests.length === 1 ? new Promise(resolve => { finishFirst = resolve; }) : Promise.resolve([]);
  });
  const older = state.refresh(true, true);
  await nextTurn();
  await state.refresh(true, false, true);
  finishFirst([]);
  await older;
  await nextTurn();
  assert.equal(requests.length, 2);
  assert.equal(requests[1].clearBenches, true);
});

test('each dataset keeps its newest success without a different dataset suppressing it', () => {
  const state = new RefreshState();
  assert.equal(state.acceptResult('spendHistory', 2, { days: 200 }), true);
  assert.equal(state.acceptResult('spend', 1, [100]), true);
  assert.equal(state.acceptResult('spendHistory', 1, { days: 100 }), false);
  assert.equal(state.acceptResult('creditTrend', 3, null), false);
  assert.equal(state.acceptResult('creditTrend', 2, { days: 200 }), true);
  assert.equal(state.acceptResult('creditTrend', 1, { days: 100 }), false);
});

test('queued requests retain full-data and manual requirements across several callers', () => {
  const state = new RefreshState();
  const first = state.request(false, true, false);
  assert.equal(first, 1);
  assert.equal(state.request(true, true, true), null);
  assert.equal(state.request(true, false, false), null);
  assert.equal(state.request(true, true, false), null);
  assert.deepEqual(state.complete(first), { usageOnly: false, interactive: true });
  assert.equal(state.busy, false);
  assert.equal(state.request(false, true, false), 2);
});

test('a saved-key waiter waits for the next attempt and late completions cannot unlock it', async () => {
  const state = new RefreshState();
  const first = state.request(false, true, false);
  let completed = false;
  const next = state.waitForNextAttempt().then(() => { completed = true; });
  state.request(true, true, false);
  state.complete(first);
  await nextTurn();
  assert.equal(completed, false);
  const second = state.request(true, true, false);
  state.complete(first);
  assert.equal(state.busy, true);
  state.complete(second);
  await next;
  assert.equal(completed, true);
  assert.equal(state.busy, false);
});
