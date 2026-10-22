import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import { providerCatalog } from '../src/providerCatalog.ts';

const source = ts.createSourceFile('main.ts', readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'), ts.ScriptTarget.Latest, true);
function load(names, bindings) {
  const functions = source.statements.filter(n => ts.isFunctionDeclaration(n) && names.includes(n.name?.text));
  assert.equal(functions.length, names.length);
  const context = vm.createContext(bindings);
  vm.runInContext(ts.transpile(functions.map(n => n.getText(source)).join('\n'), { target: ts.ScriptTarget.ES2020 }), context);
  return context;
}

test('canceling a delete with the history checkbox leaves accounts untouched', async () => {
  let calls = 0;
  const context = load(['deleteAccountById'], {
    t: key => key, appConfirm: async () => ({ ok: false, checked: true }),
    invoke: async () => { calls++; },
  });
  assert.equal(await context.deleteAccountById('tokenrhythm', 'tokenrhythm@a', 'A'), false);
  assert.equal(calls, 0);
});

test('all deletion surfaces use stable identity and the chosen history policy', async () => {
  for (const family of ['tokenrhythm', 'codex', 'cursor', 'minimax']) {
    for (const checked of [false, true]) {
      const calls = [];
      const context = load(['deleteAccountById'], {
        t: key => key, appConfirm: async () => ({ ok: true, checked }),
        invoke: async (command, args) => { calls.push({ command, ...args }); return 0; },
        forgetAccountFromBoard: () => {}, afterCredentialMutation: async () => {}, renderSettingsProviderKeys: () => {},
      });
      assert.equal(await context.deleteAccountById(family, `${family}@stable`, 'A'), true);
      assert.equal(calls.length, 1);
      assert.equal(calls[0].command, 'account_delete');
      assert.equal(calls[0].cardId, `${family}@stable`);
      assert.equal(calls[0].keepUsage, checked);
      assert.equal(calls[0].index, undefined);
    }
  }
});

test('removing a login row never erases an unrelated native login card', () => {
  for (const family of ['copilot', 'tokenrhythm', 'minimax']) {
    const id = `${family}@a`;
    const context = load(['forgetAccountFromBoard'], {
      providerDefinition: family => providerCatalog.find(p => p.familyId === family),
      accountsCache: new Map([[family, [{ id }]]]), archivedAccountsCache: new Map(),
      lastSnapshots: [{ id: family }, { id }], lastSpend: [{ id: family }, { id }],
      accountProbeSnapshots: new Map(), userSelectedAccountFor: new Map(),
      reconcileAccountLayout: () => {}, requestRender: () => {}, customizeOpen: false,
    });
    context.forgetAccountFromBoard(family, id, 0, true);
    assert.equal(context.lastSnapshots.some(row => row.id === id), false);
    assert.equal(context.lastSnapshots.some(row => row.id === family), family === 'copilot');
  }
});

test('a canceled web sign-in restores its add button and keeps existing accounts', async () => {
  const calls = [];
  const context = load(['addTokenrhythmAccount'], {
    t: key => key, invoke: async command => { calls.push(command); throw new Error('window closed'); },
    settingsToast: () => {}, afterCredentialMutation: async () => assert.fail('failed login must not mutate accounts'),
  });
  const button = { textContent: 'Add account', disabled: false };
  await context.addTokenrhythmAccount(button);
  assert.deepEqual(calls, ['tokenrhythm_login']);
  assert.equal(button.disabled, false);
  assert.equal(button.textContent, 'Add account');
});


test('an old OAuth poll cannot stop or replace a newer flow', async () => {
  for (const name of ['pollBrowserLogin', 'pollAuthOauth']) {
    for (const fails of [false, true]) {
      const old = { phase: name === 'pollBrowserLogin' ? 'browser' : 'polling', loginId: 'old', deviceAuthId: 'old' };
      const replacement = { phase: 'starting' };
      const authFlows = new Map([['codex', old]]);
      let settle;
      const context = load([name], {
        authFlows, invoke: () => new Promise((resolve, reject) => { settle = fails ? reject : resolve; }),
        stopAuthFlow: () => assert.fail('old poll canceled current flow'),
        renderAuthCenter: () => assert.fail('old poll rendered stale state'),
      });
      const task = context[name]('codex');
      authFlows.set('codex', replacement);
      settle(fails ? new Error('late failure') : { done: true, label: 'old account', error: null });
      await task;
      assert.equal(authFlows.get('codex'), replacement);
    }
  }
});


test('TokenRhythm retries use the dedicated web login and ignore obsolete outcomes', async () => {
  const authFlows = new Map();
  let settle;
  const calls = [];
  const context = load(['authAddAccount'], {
    authFlows, authFlowBusy: () => false, renderAuthCenter: () => {},
    invoke: command => { calls.push(command); return new Promise(resolve => { settle = resolve; }); },
    afterCredentialMutation: async () => {}, loadAuthCenter: async () => {},
  });
  context.authAddAccount('tokenrhythm');
  const first = authFlows.get('tokenrhythm');
  const replacement = { phase: 'starting' };
  authFlows.set('tokenrhythm', replacement);
  settle();
  await new Promise(setImmediate);
  assert.equal(authFlows.get('tokenrhythm'), replacement);
  assert.notEqual(first, replacement);
  context.authAddAccount('tokenrhythm');
  assert.deepEqual(calls, ['tokenrhythm_login', 'tokenrhythm_login']);
  settle();
  await new Promise(setImmediate);
  assert.equal(authFlows.get('tokenrhythm').phase, 'success');
});


test('default card menus resolve and capture the managed stable identity', () => {
  const context = load(['managedAccountForCard'], { providerDefinition: family => providerCatalog.find(p => p.familyId === family) });
  for (const family of ['tokenrhythm', 'minimax']) {
    const first = { id: family + '@stable-a' }, second = { id: family + '@stable-b' };
    const rows = [first, second];
    const captured = context.managedAccountForCard(family, family, rows);
    rows.reverse();
    assert.equal(captured, first);
    assert.equal(context.managedAccountForCard(family, second.id, rows), second);
  }
  for (const family of ['codex', 'cursor', 'antigravity', 'copilot', 'grok']) {
    assert.equal(context.managedAccountForCard(family, family, [{ id: family + '@managed' }]), undefined);
  }
});
