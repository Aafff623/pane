import test from 'node:test';
import assert from 'node:assert/strict';
import { fontOnboardingMode } from '../src/fontOnboarding.ts';

const fresh = { uiFont: '', uiFontPromptVersion: '', lastSeenVersion: '', welcomeDismissed: false };
test('fresh install gets education on every OS, without assuming MV Boli exists', () => {
  assert.equal(fontOnboardingMode(fresh, ['MV Boli']), 'new');
  assert.equal(fontOnboardingMode(fresh, ['Arial']), 'new');
});
test('pending first launch survives version advancement while hidden', () => {
  assert.equal(fontOnboardingMode({ ...fresh, lastSeenVersion: '0.8.5', uiFont: 'MV Boli', uiFontPromptVersion: 'pending-new' }, ['MV Boli']), 'new');
});
test('upgrade only recommends an installed font and protects custom fonts', () => {
  const old = { ...fresh, lastSeenVersion: '0.8.4', welcomeDismissed: true };
  assert.equal(fontOnboardingMode(old, ['mv boli']), 'upgrade');
  assert.equal(fontOnboardingMode(old, []), 'skip');
  assert.equal(fontOnboardingMode({ ...old, uiFont: 'Consolas' }, ['MV Boli']), 'skip');
});
test('both consent and refusal stay dismissed across subsequent upgrades', () => {
  for (const uiFont of ['', 'MV Boli']) {
    assert.equal(fontOnboardingMode({ ...fresh, uiFont, lastSeenVersion: '0.9.0', uiFontPromptVersion: '0.8.5' }, ['MV Boli']), 'skip');
  }
});

// Exercise the actual modal callback, including programmatic replacement.
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const main = ts.createSourceFile('main.ts', readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'), ts.ScriptTarget.ES2020, true);
const guideCode = ts.transpile(main.statements.filter((node) => ts.isFunctionDeclaration(node) && ['showChangelogDialog', 'maybeShowFontGuide'].includes(node.name?.text)).map((node) => node.getText(main)).join('\n'), { target: ts.ScriptTarget.ES2020 });
for (const userClose of [true, false]) {
  test(`shown guide persists user dismissal but not programmatic replacement (${userClose})`, async () => {
    const handlers = new Map();
    const overlay = { innerHTML: '', querySelector: (id) => ({ addEventListener: (event, fn) => handlers.set(id, fn) }), addEventListener() {}, remove() {} };
    const context = vm.createContext({
      pendingFontGuide: 'new', IS_PANEL_FORM: false, dismissWhatsNew: null,
      config: { uiFont: 'MV Boli', welcomeDismissed: false }, appVersion: '0.8.5',
      document: { hidden: false, querySelector: () => null, createElement: () => overlay, addEventListener() {}, removeEventListener() {}, body: { appendChild() {} } },
      getVersion: async () => '0.8.5', patchConfig: async (patch) => Object.assign(context.config, patch),
      applyAppearance() {}, maybeStartTour() {}, activeSkin: () => null, marketEntries: () => [],
      t: (key) => key, escapeHtml: (text) => text, renderChangelogBody: (body) => body, uiIcon: () => '',
    });
    vm.runInContext(guideCode, context);
    context.maybeShowFontGuide();
    if (userClose) handlers.get('#whatsnew-close')();
    else context.dismissWhatsNew();
    await new Promise((resolve) => setImmediate(resolve));
    assert.equal(context.config.welcomeDismissed, userClose);
    assert.equal(context.config.uiFontPromptVersion, userClose ? '0.8.5' : undefined);
    assert.equal(context.pendingFontGuide, userClose ? 'skip' : 'new');
  });
}
