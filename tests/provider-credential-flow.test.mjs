import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import { credentialFailure, accountFormReady, enabledAccountCount } from '../src/credentialPolicy.ts';
import { providerCatalog } from '../src/providerCatalog.ts';
import { MECHANISMS } from '../src/providerMechanisms.ts';
import { RefreshState } from '../src/refreshState.ts';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
function actualFunctions(names) {
  const functions = ast.statements.filter((node) => ts.isFunctionDeclaration(node) && names.includes(node.name?.text));
  assert.equal(functions.length, names.length);
  return ts.transpile(functions.map((node) => node.getText(ast)).join('\n'), { target: ts.ScriptTarget.ES2020 });
}
test('all catalog providers have concrete help and documented boundaries', () => {
  for (const provider of providerCatalog) {
    const help = MECHANISMS[provider.familyId];
    assert.ok(help, provider.familyId);
    assert.ok(help.provides?.length, provider.familyId);
    assert.ok(help.limitsKey, provider.familyId);
  }
  for (const family of ['bocha', 'tavily', 'firecrawl', 'brave']) assert.deepEqual(MECHANISMS[family].env ?? [], []);
});
test('saving validates required fields independently of live query success', () => {
  assert.equal(accountFormReady('deliberately-invalid-key', 'My test account', '', false), true);
  assert.equal(accountFormReady('key', '', '', false), false);
  assert.equal(accountFormReady('key', 'Relay', '', true), false);
  assert.equal(enabledAccountCount([{ id: 'a' }, { id: 'b' }], ['b']), 1);
});
test('network, credentials, permissions and exhaustion-like rate limits are distinct', () => {
  assert.equal(credentialFailure('network timeout'), 'network');
  assert.equal(credentialFailure('HTTP 401'), 'credential');
  assert.equal(credentialFailure('HTTP 403'), 'permission');
  assert.equal(credentialFailure('HTTP 429'), 'rateLimit');
  assert.equal(credentialFailure('unexpected body'), 'unknown');
});

test('diagnostic ids and session field names do not prove credential failure', () => {
  assert.equal(credentialFailure('unexpected response; request id 142903'), 'unknown');
  assert.equal(credentialFailure('cookie field absent from quota response'), 'unknown');
  assert.equal(credentialFailure('HTTP 503: access token check failed'), 'network');
  assert.equal(credentialFailure('session expired'), 'credential');
});

for (const healthy of [true, false]) {
  test(`saving the eighth account immediately updates the real overview count (${healthy ? 'connected' : 'failed query'})`, async () => {
    const existing = Array.from({ length: 7 }, (_, i) => ({ id: `clinepass@${i}`, label: `Account ${i}` }));
    const added = { id: 'clinepass@new', label: 'New account' };
    const snapshots = existing.map((entry, i) => ({ id: i === 0 ? 'clinepass' : entry.id, status: 'ok', metrics: [] }));
    let painted;
    const context = vm.createContext({
      config: { disabled: ['clinepass'], removedProviders: [], layout: { providers: {} } },
      accountsCache: new Map([['clinepass', existing]]),
      accountProbeSnapshots: new Map([['clinepass', { status: healthy ? 'ok' : 'error', metrics: [], error: healthy ? null : 'HTTP 401' }]]),
      lastSnapshots: snapshots, recentlyKeyed: new Map(), refreshState: new RefreshState(), IS_PANEL_FORM: false,
      enabledAccountCount, providerFamily: (id) => id.split('@')[0], providerDisplayName: (family) => family,
      patchConfig: async (patch) => Object.assign(context.config, patch),
      refreshSettingsProviderKey: async () => context.accountsCache.set('clinepass', [...existing, added]),
      reconcileAccountLayout: () => false, isCardDisabled: (id) => context.config.disabled.includes(id),
      isBareHintCard: () => false,
      accountHealthDot: (id) => context.lastSnapshots.find((snapshot) => snapshot.id === id)?.status === 'ok' ? 'green' : 'error',
      requestRender: () => { painted = context.accountAvailabilitySummary('clinepass'); },
      forceUsageRefreshAttempt: async () => {}, requestTraySync: () => {}, t: (key) => key,
    });
    vm.runInContext(actualFunctions(['afterCredentialMutation', 'overviewAccountSnapshots', 'accountAvailabilitySummary']), context);
    await context.afterCredentialMutation('clinepass', added.id);
    assert.equal(painted.total, 8);
    assert.equal(painted.available, healthy ? 8 : 7);
    assert.equal(context.lastSnapshots.at(-1).status, healthy ? 'ok' : 'error');
    assert.equal(context.config.disabled.length, 0);
  });
}
