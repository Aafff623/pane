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
  const context = vm.createContext({
    isWebSessionFamily: family => providerCatalog.find(p => p.familyId === family)?.webSession === true,
    ...bindings,
  });
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
      config: {}, patchConfig: async () => {},
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
  const context = load(['addWebSessionAccount'], {
    t: key => key, invoke: async (command, args) => { calls.push({ command, ...args }); throw new Error('window closed'); },
    settingsToast: () => {}, afterCredentialMutation: async () => assert.fail('failed login must not mutate accounts'),
    providerDisplayName: family => family,
  });
  const button = { textContent: 'Add account', disabled: false };
  await context.addWebSessionAccount(button, 'tokenrhythm');
  assert.deepEqual(calls, [{ command: 'web_session_login', family: 'tokenrhythm' }]);
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
    invoke: (command, args) => { calls.push({ command, ...args }); return new Promise(resolve => { settle = resolve; }); },
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
  assert.deepEqual(calls, [
    { command: 'web_session_login', family: 'tokenrhythm' },
    { command: 'web_session_login', family: 'tokenrhythm' },
  ]);
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


test('default account notes follow credential identity after deletion and promotion', () => {
  const accountsCache = new Map([['minimax', [{ id: 'minimax@a', label: '' }, { id: 'minimax@b', label: '11-4' }]]]);
  const config = { layout: { providers: { 'minimax@a': { note: 'Old subscription' } } } };
  const context = load(['normalizeNote', 'managedAccountForCard', 'accountNoteKeys', 'accountNoteKey', 'accountNote', 'cardNote'], {
    accountsCache, config, providerDefinition: family => providerCatalog.find(p => p.familyId === family),
    providerFamily: id => id.split('@')[0], NOTE_MAX_CODE_POINTS: 48,
  });
  assert.equal(context.accountNoteKey('minimax'), 'minimax@a');
  assert.equal(context.accountNote('minimax'), 'Old subscription');
  accountsCache.set('minimax', [{ id: 'minimax@b', label: '11-4' }]);
  assert.equal(context.accountNoteKey('minimax'), 'minimax@b');
  assert.equal(context.accountNote('minimax'), '');
});

test('layout reconciliation keeps account notes and prunes pins', () => {
  const config = {disabled:[],layout:{providerOrder:['minimax','minimax@deleted'],providers:{
    minimax:{pinnedAccount:'minimax@deleted'},'minimax@a':{note:'A'},'minimax@deleted':{note:'Deleted'},
  }}};
  const context = load(['reconcileAccountLayout'], {config,lastSnapshots:[],lastSpend:[],
    providerDefinition:family=>providerCatalog.find(p=>p.familyId===family), providerFamily:id=>id.split('@')[0],patchConfig:async()=>{},
  });
  context.reconcileAccountLayout('minimax',[{id:'minimax@a'}]);
  assert.equal(config.layout.providers['minimax@a'].note,'A');
  // Notes are identity-bound: archive/delete/restore must not lose them.
  assert.equal(config.layout.providers['minimax@deleted'].note,'Deleted');
  assert.equal(config.layout.providers.minimax.pinnedAccount,undefined);
});


test('local CLI account notes remain separate from the provider title', () => {
  const accountsCache = new Map([['codex', [{id:'codex',label:'CLI'}, {id:'codex@managed',label:'Managed'}]]]);
  const context = load(['managedAccountForCard','accountNoteKey'], {
    accountsCache,providerDefinition:family=>providerCatalog.find(p=>p.familyId===family),
  });
  assert.equal(context.accountNoteKey('codex'),'codex@__default__');
  assert.equal(context.accountNoteKey('codex@managed'),'codex@managed');
});


test('deleting the folded default drops its pin without touching a sibling pin', () => {
  for (const pinned of ['minimax', 'minimax@a', 'minimax@b']) {
    const config = {layout:{providers:{minimax:{pinnedAccount:pinned}}}};
    const context = load(['forgetAccountFromBoard'], {
      config,patchConfig:async()=>{}, providerDefinition:family=>providerCatalog.find(p=>p.familyId===family),
      accountsCache:new Map([['minimax',[{id:'minimax@a'},{id:'minimax@b'}]]]),archivedAccountsCache:new Map(),
      lastSnapshots:[{id:'minimax'},{id:'minimax@b'}],lastSpend:[],accountProbeSnapshots:new Map(),userSelectedAccountFor:new Map(),
      reconcileAccountLayout:()=>{},requestRender:()=>{},customizeOpen:false,
    });
    context.forgetAccountFromBoard('minimax','minimax@a',0,true);
    assert.equal(config.layout.providers.minimax.pinnedAccount,pinned==='minimax@b'?pinned:undefined);
  }
});


test('the customize account note edits the same display note the card reads', () => {
  const list = [{id:'tokenrhythm@a',label:'我'},{id:'tokenrhythm@b',label:'老马'}];
  const accountsCache = new Map([['tokenrhythm',list]]);
  const config = {layout:{providers:{'tokenrhythm@__default__':{note:'旧备注'}}}};
  const context = load(['normalizeNote','managedAccountForCard','accountNoteKeys','accountNoteKey','accountNote','labelForAccount','cardNote','setAccountNote'], {
    accountsCache,config,patchConfig:async()=>{},requestRender:()=>{},customizeOpen:false,renderDrawerBody:()=>{},NOTE_MAX_CODE_POINTS:48,emitNoteUpdate:()=>{},
    providerDefinition:family=>providerCatalog.find(p=>p.familyId===family),
    providerFamily:id=>id.split('@')[0], t:(key,{n})=>`${key}:${n}`, providerDisplayName:()=>'基元律动',
  });
  // A note written before the stable id existed is still readable.
  assert.equal(context.accountNote('tokenrhythm@a'), '旧备注');
  context.setAccountNote('tokenrhythm@a', '新备注');
  assert.equal(config.layout.providers['tokenrhythm@a'].note, '新备注');
  assert.equal(config.layout.providers['tokenrhythm@__default__'].note, undefined);
  assert.equal(context.labelForAccount('tokenrhythm@a', list), '新备注');
});


test('missing account names never fall back to a hash or masked token', () => {
  const list = [{id:'minimax@a',label:'',email:''}];
  const context = load(['normalizeNote','managedAccountForCard','accountNoteKeys','accountNote','cardNote','labelForAccount'], {
    accountsCache:new Map([['minimax',list]]),config:{layout:{providers:{}}}, NOTE_MAX_CODE_POINTS:48,
    providerDefinition:family=>providerCatalog.find(p=>p.familyId===family),
    providerFamily:id=>id.split('@')[0], t:(key,{n})=>`${key}:${n}`, providerDisplayName:()=>'MiniMax',
  });
  assert.equal(context.labelForAccount('minimax@a', list), 'customize.acctDefaultName:1');
});


test('overview detail names prefer the shown account note', () => {
  const list = [{id:'minimax@a',label:''},{id:'minimax@b',label:''}];
  const accountsCache = new Map([['minimax',list]]);
  const config = {layout:{providers:{'minimax@a':{note:'主账号'}}}};
  const context = load(['normalizeNote','shownDisplayName','labelForAccount','accountNote','accountNoteKeys','managedAccountForCard','cardNote','notedName'], {
    accountsCache,config,NOTE_MAX_CODE_POINTS:48,providerDefinition:family=>providerCatalog.find(p=>p.familyId===family),
    providerFamily:id=>id.split('@')[0], t:(key,{n})=>`${key}:${n}`, providerDisplayName:()=>'MiniMax',
  });
  const card = {id:'minimax'};
  assert.equal(context.shownDisplayName(card, {id:'minimax'}, 'minimax', 'MiniMax'), '主账号');
  assert.equal(context.shownDisplayName(card, {id:'minimax@b'}, 'minimax', 'MiniMax'), 'customize.acctDefaultName:2');
});


test('display notes are single-line and capped by code points', () => {
  const context = load(['normalizeNote'], {NOTE_MAX_CODE_POINTS:48});
  assert.equal(context.normalizeNote('  新   备注\n第二行  '), '新 备注 第二行');
  assert.equal([...context.normalizeNote('好'.repeat(60))].length, 48);
});


test('OAuth device families route Customize to Settings instead of a second login UI', () => {
  const context = load(['renderSettingsAuthJump'], {
    t:key=>key, escapeHtml:value=>String(value), credChipsHtml:()=>'<chips>',
    custNoteField:()=>'', custApplyHtml:()=>'', groupPickerHtml:()=>'',
  });
  for (const family of ['codex', 'grok', 'copilot', 'kiro', 'windsurf', 'trae', 'zed']) {
    const html = context.renderSettingsAuthJump(family);
    assert.match(html, new RegExp(`data-open-keys-page="${family}"`));
    assert.ok(!html.includes('data-oauth-login'));
    assert.ok(!html.includes('data-login-family'));
  }
});


test('one saved key cannot become two tabs through a bare alias or deleted snapshot', () => {
  const context = load(['overviewAccountSnapshots'], {
    accountsCache:new Map([['minimax',[{id:'minimax@live'}]]]),
    lastSnapshots:[{id:'minimax'},{id:'minimax@live'},{id:'minimax@deleted'}],
    providerDefinition:family=>providerCatalog.find(p=>p.familyId===family),
    providerFamily:id=>id.split('@')[0], isCardDisabled:()=>false,isBareHintCard:()=>false,
  });
  assert.deepEqual(Array.from(context.overviewAccountSnapshots('minimax'),row=>row.id),['minimax']);
});


test('an env or CLI backed bare provider remains visible with no saved accounts', () => {
  const context = load(['overviewAccountSnapshots'], {
    accountsCache:new Map([['minimax',[]]]),lastSnapshots:[{id:'minimax',status:'ok'},{id:'minimax@deleted',status:'ok'}],
    providerDefinition:family=>providerCatalog.find(p=>p.familyId===family),providerFamily:id=>id.split('@')[0],
    isCardDisabled:()=>false,isBareHintCard:()=>false,
  });
  assert.deepEqual(Array.from(context.overviewAccountSnapshots('minimax'),row=>row.id),['minimax']);
});


test('deleting a One/New API key removes its live card immediately', () => {
  const accountsCache = new Map([['onenewapi', [{ id: 'onenewapi@api1' }, { id: 'onenewapi@api2' }]]]);
  const lastSpend = [{ id: 'onenewapi@api1' }, { id: 'onenewapi@api2' }];
  const context = load(['forgetOneNewApiCards', 'onaSnapshotId'], {
    ONA_FAMILY: 'onenewapi',
    lastSnapshots: [{ id: 'onenewapi@api1' }, { id: 'onenewapi@api2' }],
    lastSpend, accountsCache, userSelectedAccountFor: new Map([['onenewapi', 'onenewapi@api2']]),
    requestRender: () => {}, customizeOpen: false, renderDrawerBody: () => {},
  });
  context.forgetOneNewApiCards(['api2']);
  assert.deepEqual(context.lastSnapshots.map((row) => row.id), ['onenewapi@api1']);
  assert.deepEqual(context.lastSpend.map((row) => row.id), ['onenewapi@api1']);
  assert.deepEqual(accountsCache.get('onenewapi').map((row) => row.id), ['onenewapi@api1']);
  assert.equal(context.userSelectedAccountFor.has('onenewapi'), false);
});
