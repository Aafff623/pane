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

test('a completed query with no quota data is its own state', () => {
  assert.equal(credentialFailure('Step Plan returned no quota data'), 'empty');
  assert.equal(credentialFailure('usage endpoint returned no windows'), 'empty');
  assert.equal(credentialFailure('API discontinued'), 'unsupported');
  assert.equal(credentialFailure('session expired'), 'credential');
});

test('subscription expiry is not flattened into sign-in or unknown', () => {
  assert.equal(credentialFailure('subscription expired'), 'subscription');
  assert.equal(credentialFailure('no active subscription'), 'subscription');
  assert.equal(credentialFailure('membership cancelled'), 'subscription');
  assert.equal(credentialFailure('套餐已到期，请续费'), 'subscription');
  assert.equal(credentialFailure('session expired'), 'credential');
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
      providerDefinition: (family) => providerCatalog.find((p) => p.familyId === family),
      enabledAccountCount, providerFamily: (id) => id.split('@')[0], providerDisplayName: (family) => family,
      patchConfig: async (patch) => Object.assign(context.config, patch),
      refreshSettingsProviderKey: async () => context.accountsCache.set('clinepass', [...existing, added]),
      reconcileAccountLayout: () => false, isCardDisabled: (id) => context.config.disabled.includes(id),
      isBareHintCard: () => false,
      accountHealthDot: (id) => context.lastSnapshots.find((snapshot) => snapshot.id === id)?.status === 'ok' ? 'green' : 'error',
      requestRender: () => { painted = context.accountAvailabilitySummary('clinepass'); },
      forceUsageRefreshAttempt: async () => {}, requestTraySync: () => {}, t: (key) => key,
      notifyAccountsChanged: () => {},
    });
    vm.runInContext(actualFunctions(['afterCredentialMutation', 'overviewAccountSnapshots', 'accountAvailabilitySummary']), context);
    await context.afterCredentialMutation('clinepass', added.id);
    assert.equal(painted.total, 8);
    assert.equal(painted.available, healthy ? 8 : 7);
    assert.equal(context.lastSnapshots.at(-1).status, healthy ? 'ok' : 'error');
    assert.equal(context.config.disabled.length, 0);
  });
}

test('overview tiles show a distinct label per failure kind', () => {
  const code = actualFunctions(['overviewFailureLabel']);
  const context = vm.createContext({ t: (key) => key, credentialFailure });
  vm.runInContext(code, context);
  assert.equal(context.overviewFailureLabel({ status: 'error', error: 'session expired' }), 'overview.needsLogin');
  assert.equal(context.overviewFailureLabel({ status: 'error', error: 'subscription expired' }), 'overview.failure.subscription');
  assert.equal(context.overviewFailureLabel({ status: 'error', error: 'network timeout' }), 'overview.failure.network');
  assert.equal(context.overviewFailureLabel({ status: 'error', error: 'HTTP 429' }), 'overview.failure.rateLimit');
  assert.equal(context.overviewFailureLabel({ status: 'error', error: 'HTTP 403' }), 'overview.failure.permission');
  assert.equal(context.overviewFailureLabel({ status: 'error', error: 'no quota data' }), 'overview.failure.empty');
  assert.equal(context.overviewFailureLabel({ status: 'no_credentials', error: null }), 'overview.needsCredentials');
  assert.equal(context.overviewFailureLabel({ status: 'needs_setup', error: 'anything' }), 'overview.needsSetup');
});

test('repair views keep action, status and steps aligned with the failure kind', () => {
  const code = actualFunctions(['credentialMethod', 'repairView']);
  const context = vm.createContext({
    MECHANISMS: { claude: { credential: 'local' }, opencode: { credential: 'key' } },
    providerDefinition: () => ({ supportsApiKey: true }),
  });
  vm.runInContext(code, context);
  const credential = context.repairView('credential', 'claude', false, true);
  assert.equal(credential.statusKey, 'quotaState.status.signIn');
  assert.equal(credential.primaryAction, 'setup');
  assert.match(credential.steps[0], /credential\.local\.1/);
  const key = context.repairView('credential', 'opencode', false, true);
  assert.match(key.steps[0], /credential\.key\.1/);
  const empty = context.repairView('empty', 'opencode', false, true);
  assert.equal(empty.statusKey, 'quotaState.status.empty');
  assert.equal(empty.primaryAction, 'refresh');
  const network = context.repairView('network', 'opencode', false, true);
  assert.equal(network.primaryAction, 'refresh');
  assert.equal(network.tone, 'warn');
  const subscription = context.repairView('subscription', 'opencode', true, true);
  assert.equal(subscription.primaryAction, 'link');
  const install = context.repairView('not_installed', 'claude', false, false);
  assert.equal(install.primaryAction, 'help');
  const localNoPane = context.repairView('credential', 'claude', false, false);
  assert.equal(localNoPane.primaryAction, 'help', 'local CLIs must not pretend Pane can sign in for them');
  assert.equal(localNoPane.primaryKey, 'quotaState.action.loginHelp');
});

test('repair kind understands installed, missing and failure states', () => {
  const code = actualFunctions(['repairKind']);
  const context = vm.createContext({ credentialFailure });
  vm.runInContext(code, context);
  assert.equal(context.repairKind({ status: 'needs_setup', error: '' }, undefined), 'needs_setup');
  assert.equal(context.repairKind({ status: 'no_credentials' }, { installState: 'installed' }), 'not_signed_in');
  assert.equal(context.repairKind({ status: 'no_credentials' }, { installState: 'missing' }), 'not_installed');
  assert.equal(context.repairKind({ status: 'error', error: 'Step Plan returned no quota data' }), 'empty');
  assert.equal(context.repairKind({ status: 'error', error: 'HTTP 401' }), 'credential');
});
