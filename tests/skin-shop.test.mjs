import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

// Compile the real shop helpers out of main.ts, with the registry and copy
// stubbed. The point is the contract: entry states, factual copy, escaping.
const main = ts.createSourceFile(
  'main.ts',
  readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'),
  ts.ScriptTarget.ES2020,
  true,
);
const names = new Set([
  'skinShopTagline',
  'skinShopEntries',
  'skinShopEntry',
  'skinShopPackSize',
  'skinShopCountLabel',
  'skinShopStateLabel',
  'skinShopSubtitle',
  'skinShopSample',
  'skinShopPlaceholder',
  'renderSkinShopCard',
  'renderSkinShopGallery',
  'downloadSkinShopEntry',
  'removeSkinShopEntry',
  'renderSkinShopActions',
  'renderSkinShopMeta',
]);
const code = ts.transpile(
  main.statements
    .filter((node) => ts.isFunctionDeclaration(node) && names.has(node.name?.text))
    .map((node) => node.getText(main))
    .join('\n'),
  { target: ts.ScriptTarget.ES2020 },
);

const dict = {
  'skin.nativeName': 'Native',
  'skin.nativeTagline': 'The app’s own look.',
  'skin.signalTagline': 'Hand-drawn stamps and a cat courier.',
  'skin.inkTagline': 'Ink-washed mountains.',
  'skin.shopBundlePackage': 'Skin package',
  'skin.shopCount': '{n} skins',
  'skin.shopCountOne': '1 skin',
  'skin.applied': 'Applied',
  'skin.installed': 'Installed',
  'skin.remoteState': 'Available',
  'skin.shopUpdateBadge': 'Update',
  'skin.shopOpen': 'Open {name}',
  'skin.shopVersion': 'Version',
  'skin.shopStatus': 'Status',
  'skin.shopSource': 'Source',
  'skin.shopOfficial': 'Official index',
  'skin.shopLocal': 'Installed locally',
  'skin.shopSize': 'Package',
  'skin.download': 'Download',
  'skin.downloading': 'Downloading…',
  'skin.use': 'Use this skin',
  'skin.remove': 'Remove',
  'skin.useNative': 'Use native appearance',
  'skin.inUse': 'In use',
  'skin.builtIn': 'Built-in',
  'skin.shopMissing': 'Not installed',
  'skin.shopSelected': 'Selected',
  'skin.shopMissingNote': 'The selected skin is not installed.',
  'skin.shopRegistryFailed': 'Could not re-read the installed packs.',
  'skin.downloadFailed': 'Skin pack download failed.',
  'skin.removeFailed': 'Could not remove the skin pack.',
  'skin.saveFailed': 'Could not save skin selection',
  'skin.market': 'Skin workshop',
  'skin.shopDesc': 'Dress the panel.',
  'skin.shopLayout': 'Gallery layout',
  'skin.shopMasonry': 'Masonry',
  'skin.shopGrid': 'Grid',
  'skin.shopSearch': 'Search skins',
  'skin.shopRefresh': 'Refresh index',
  'skin.shopLoading': 'Checking for skins…',
  'skin.shopOffline': 'The index could not be reached.',
  'skin.indexError.network': 'Network request failed',
  'skin.shopNoResults': 'No skins match “{q}”.',
};

function makeContext(state) {
  const config = { skinId: state.saved ?? null };
  return vm.createContext({
    SKIN_SHOP_NATIVE: '__native',
    activeSkin: () => state.active,
    config,
    normalizeSkin: (value) => typeof value === 'string' ? value : null,
    packSkinList: () => state.packs,
    remotePackIndex: () => state.index,
    skinDownloading: null,
    skinShopSelected: null,
    skinShopFeedback: state.feedback ?? null,
    skinIndexSettled: () => state.settled ?? true,
    skinIndexRefreshing: () => state.refreshing ?? false,
    remoteIndexErrorClass: () => state.indexError ?? null,
    skinShopView: 'grid',
    skinShopSearch: state.search ?? '',
    renderSkinShop() {},
    invoke: async (command, args) => {
      (state.invokes ??= []).push([command, args]);
      if (state.failInvoke) throw new Error('command failed');
      return null;
    },
    refreshInstalledPacks: async () => {
      state.refreshes = (state.refreshes ?? 0) + 1;
      return state.registryOk ?? true;
    },
    applySkin: () => { state.applied = (state.applied ?? 0) + 1; },
    broadcastSkinPackChange: () => { state.broadcasts = (state.broadcasts ?? 0) + 1; },
    selectSkin: async (id) => {
      (state.selects ??= []).push(id);
      if (!state.selectFails) config.skinId = id;
    },
    isNewerVersion: (candidate, installed) => {
      const a = candidate.split('.').map(Number);
      const b = installed.split('.').map(Number);
      for (let i = 0; i < Math.max(a.length, b.length); i += 1) {
        if ((a[i] ?? 0) !== (b[i] ?? 0)) return (a[i] ?? 0) > (b[i] ?? 0);
      }
      return false;
    },
    t: (key, vars = {}) => {
      let text = dict[key] ?? key;
      for (const [name, value] of Object.entries(vars)) text = text.split(`{${name}}`).join(String(value));
      return text;
    },
    escapeHtml: (value) => String(value).replace(/[&<>"']/g, (char) => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
    }[char])),
    uiIcon: () => '',
    renderSkinSample: () => '<div class="skin-sample"></div>',
  });
}

function boot(state) {
  const context = makeContext(state);
  vm.runInContext(code, context);
  return context;
}

const comic = { id: 'comic-pixel', name: 'Comic Pixel', version: '1.0.2' };
const signal = { id: 'signal', name: 'Cloud Post Office', version: '1.0.2' };

test('the applied skin comes first and native stays a first-class row', () => {
  const ctx = boot({
    active: comic,
    saved: 'comic-pixel',
    saved: 'comic-pixel',
    packs: [comic],
    index: [
      { id: 'comic-pixel', name: 'Comic Pixel', version: '1.0.3', size: 109652, url: 'u', sha256: 's' },
      { id: 'signal', name: 'Cloud Post Office', version: '1.0.2', size: 1528012, url: 'u', sha256: 's' },
    ],
  });
  const entries = ctx.skinShopEntries();
  assert.equal(entries[0].id, 'comic-pixel');
  assert.equal(entries[0].applied, true);
  assert.equal(entries[0].updateAvailable, true);
  assert.equal(entries[0].size, 109652);
  assert.equal(entries.find((entry) => entry.id === '__native').applied, false);
  assert.equal(entries.find((entry) => entry.id === 'signal').installed, false);
  assert.equal(entries.filter((entry) => entry.id === 'comic-pixel').length, 1);
});

test('native is applied with no pack, and a real pack named native cannot shadow it', () => {
  const ctx = boot({
    active: null,
    packs: [{ id: 'native', name: 'A local pack named native', version: '1.0.0' }],
    index: [],
  });
  const entries = ctx.skinShopEntries();
  assert.equal(entries[0].id, '__native');
  assert.equal(entries[0].applied, true);
  assert.equal(entries.filter((entry) => entry.id === 'native').length, 1);
  assert.equal(entries.find((entry) => entry.id === 'native').applied, false);
});

test('subtitles use verified copy or a factual package line, never invented copy', () => {
  const ctx = boot({
    active: null,
    packs: [comic],
    index: [{ id: 'signal', name: 'Cloud Post Office', version: '1.0.2', size: 1, url: 'u', sha256: 's' }],
  });
  const entries = ctx.skinShopEntries();
  assert.equal(ctx.skinShopSubtitle(entries.find((entry) => entry.id === '__native')), 'The app’s own look.');
  assert.equal(ctx.skinShopSubtitle(entries.find((entry) => entry.id === 'comic-pixel')), 'Skin package · v1.0.2');
  assert.equal(ctx.skinShopSubtitle(entries.find((entry) => entry.id === 'signal')), 'Hand-drawn stamps and a cat courier.');
});

test('state, count and size labels are exact', () => {
  const ctx = boot({ active: comic, saved: 'comic-pixel', packs: [comic], index: [] });
  const [comicEntry, nativeEntry] = ['comic-pixel', '__native'].map((id) => ctx.skinShopEntry(id));
  assert.equal(ctx.skinShopStateLabel(comicEntry), 'Applied');
  assert.equal(ctx.skinShopStateLabel(nativeEntry), 'Built-in');
  assert.equal(ctx.skinShopCountLabel(1), '1 skin');
  assert.equal(ctx.skinShopCountLabel(4), '4 skins');
  assert.equal(ctx.skinShopPackSize(1528012), '1.5 MB');
});

test('card markup escapes pack names and taglines', () => {
  const ctx = boot({
    active: null,
    packs: [{ id: 'evil', name: '<img src=x onerror=alert(1)>', version: '1.0.0' }],
    index: [],
  });
  const html = ctx.renderSkinShopCard(ctx.skinShopEntry('evil'));
  assert.ok(!html.includes('<img src=x'));
  assert.ok(html.includes('&lt;img src=x'));
  assert.ok(html.includes('Open &lt;img src=x onerror=alert(1)&gt;'));
});

test('an applied card shows In use + Installed instead of repeating Applied', () => {
  const ctx = boot({
    active: comic,
    saved: 'comic-pixel',
    packs: [comic],
    index: [{ id: 'comic-pixel', name: 'Comic Pixel', version: '1.0.3', size: 1, url: 'u', sha256: 's' }],
  });
  const html = ctx.renderSkinShopCard(ctx.skinShopEntry('comic-pixel'));
  assert.ok(html.includes('skin-card-badge">In use'));
  assert.ok(html.includes('skin-chip is-applied">Installed'));
  assert.ok(html.includes('skin-card-badge is-update">Update'));
});

test('a saved pack that is missing locally is selected, not silently native', () => {
  const ctx = boot({
    active: null,
    saved: 'signal',
    packs: [],
    index: [{ id: 'signal', name: 'Cloud Post Office', version: '1.0.2', size: 1, url: 'u', sha256: 's' }],
  });
  const entries = ctx.skinShopEntries();
  const native = entries.find((entry) => entry.id === '__native');
  const signal = entries.find((entry) => entry.id === 'signal');
  assert.equal(native.applied, false);
  assert.equal(signal.selected, true);
  assert.equal(ctx.skinShopStateLabel(signal), 'Not installed');
});

test('actions map to the real registry state and keep native restore available', () => {
  const remote = boot({
    active: null,
    packs: [],
    index: [{ id: 'signal', name: 'Cloud Post Office', version: '1.0.2', size: 1, url: 'u', sha256: 's' }],
  });
  assert.match(remote.renderSkinShopActions(remote.skinShopEntry('signal')), /data-skin-shop-download="signal"/);

  const installed = boot({ active: null, packs: [comic], index: [] });
  const installedHtml = installed.renderSkinShopActions(installed.skinShopEntry('comic-pixel'));
  assert.match(installedHtml, /data-skin-shop-select="comic-pixel"/);
  assert.match(installedHtml, /data-skin-shop-remove="comic-pixel"/);
  assert.ok(!installedHtml.includes('data-skin-shop-reset'));

  const applied = boot({ active: comic, saved: 'comic-pixel', packs: [comic], index: [] });
  const appliedHtml = applied.renderSkinShopActions(applied.skinShopEntry('comic-pixel'));
  assert.match(appliedHtml, /data-skin-shop-select="comic-pixel" disabled/);
  assert.match(appliedHtml, /data-skin-shop-remove="comic-pixel" disabled/);
  assert.match(appliedHtml, /data-skin-shop-reset/);

  const native = boot({ active: null, packs: [], index: [] });
  assert.match(native.renderSkinShopActions(native.skinShopEntry('__native')), /data-skin-shop-reset disabled/);
});

test('download never repaints a stale registry and reports the failure', async () => {
  const state = { active: null, saved: 'signal', packs: [], index: [], registryOk: false };
  const ctx = boot(state);
  await ctx.downloadSkinShopEntry({ id: 'signal', remote: { url: 'u', sha256: 's' } });
  assert.equal(ctx.skinShopFeedback, 'Could not re-read the installed packs.');
  assert.equal(state.refreshes, 1);
  assert.equal(state.applied, undefined);
  assert.equal(state.broadcasts, 1);
});

test('download applies the saved selection only on a successful refresh', async () => {
  const state = { active: null, saved: 'signal', packs: [], index: [], registryOk: true };
  const ctx = boot(state);
  await ctx.downloadSkinShopEntry({ id: 'signal', remote: { url: 'u', sha256: 's' } });
  assert.equal(state.applied, 1);
  assert.equal(state.refreshes, 1);
  assert.equal(ctx.skinShopFeedback, null);
});

test('download failure keeps the generic reason visible and does not refresh', async () => {
  const state = { active: null, saved: null, packs: [], index: [], failInvoke: true };
  const ctx = boot(state);
  await ctx.downloadSkinShopEntry({ id: 'signal', remote: { url: 'u', sha256: 's' } });
  assert.equal(ctx.skinShopFeedback, 'Skin pack download failed.');
  assert.equal(state.refreshes, undefined);
  assert.equal(state.broadcasts, undefined);
});

test('removing the selected pack clears the saved id before deleting', async () => {
  const state = { active: { id: 'signal' }, saved: 'signal', packs: [], index: [], registryOk: true };
  const ctx = boot(state);
  await ctx.removeSkinShopEntry('signal');
  assert.deepEqual(state.selects, [null]);
  assert.equal(state.invokes[0][0], 'skin_pack_remove');
  assert.equal(state.refreshes, 1);
  assert.equal(ctx.skinShopFeedback, null);
});

test('removal aborts if the saved selection cannot be cleared', async () => {
  const state = { active: { id: 'signal' }, saved: 'signal', packs: [], index: [], selectFails: true };
  const ctx = boot(state);
  await ctx.removeSkinShopEntry('signal');
  assert.equal(ctx.skinShopFeedback, 'Could not save skin selection');
  assert.equal(state.invokes, undefined);
});

test('removal reports a failed registry read but still removes the files', async () => {
  const state = { active: null, saved: null, packs: [], index: [], registryOk: false };
  const ctx = boot(state);
  await ctx.removeSkinShopEntry('signal');
  assert.equal(ctx.skinShopFeedback, 'Could not re-read the installed packs.');
  assert.equal(state.invokes[0][0], 'skin_pack_remove');
  assert.equal(state.broadcasts, 1);
});

test('the gallery explains a missing saved skin and surfaces action failures', () => {
  const ctx = boot({
    active: null,
    saved: 'signal',
    packs: [],
    index: [{ id: 'signal', name: 'Cloud Post Office', version: '1.0.2', size: 1, url: 'u', sha256: 's' }],
    feedback: 'Skin pack download failed.',
  });
  const html = ctx.renderSkinShopGallery(ctx.skinShopEntries());
  assert.ok(html.includes('The selected skin is not installed.'));
  assert.ok(html.includes('Skin pack download failed.'));
  assert.ok(html.includes('Not installed'));
});

test('the gallery shows checking and offline states, and disables refresh while in flight', () => {
  const loading = boot({ active: null, packs: [], index: [], refreshing: true });
  const loadingHtml = loading.renderSkinShopGallery(loading.skinShopEntries());
  assert.ok(loadingHtml.includes('Checking for skins…'));
  assert.match(loadingHtml, /data-skin-shop-refresh disabled/);

  const offline = boot({ active: null, packs: [], index: [], indexError: 'network' });
  const offlineHtml = offline.renderSkinShopGallery(offline.skinShopEntries());
  assert.ok(offlineHtml.includes('The index could not be reached.'));
  assert.ok(offlineHtml.includes('Network request failed'));
});

test('metadata marks official vs local sources and shows size only from the index', () => {
  const official = boot({
    active: null,
    packs: [comic],
    index: [{ id: 'comic-pixel', name: 'Comic Pixel', version: '1.0.2', size: 109652, url: 'u', sha256: 's' }],
  });
  const officialHtml = official.renderSkinShopMeta(official.skinShopEntry('comic-pixel'));
  assert.ok(officialHtml.includes('Official index'));
  assert.ok(officialHtml.includes('0.1 MB'));

  const local = boot({ active: null, packs: [comic], index: [] });
  const localHtml = local.renderSkinShopMeta(local.skinShopEntry('comic-pixel'));
  assert.ok(localHtml.includes('Installed locally'));
  assert.ok(!localHtml.includes('Package'));
});
