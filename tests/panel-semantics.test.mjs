import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const main = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const i18n = readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
const count = (haystack, needle) => haystack.split(needle).length - 1;

test('quota percentages keep one labelled convention', () => {
  assert.match(main, /function quotaPctLabel\(usedPercent: number \| null\): string/);
  assert.match(main, /const shown = config\.showUsed \? used : 100 - used;/);
  assert.match(main, /style="width:\$\{config\.showUsed \? used : left\}%"/);
  assert.match(main, /const pctLabel = pct === null \? null : quotaPctLabel\(pct\);/);
  assert.match(main, /fold-main-quota">\$\{escapeHtml\(headLabel/);
  assert.doesNotMatch(main, /Math\.round\(headPct\)/);
});

test('the fake save state stays gone and the reload action stays reachable', () => {
  assert.doesNotMatch(main, /settingsDirty|markDirty|pendingJumpAnimation/);
  assert.match(main, /settings\.applyReload/);
  assert.match(main, /event\.key\.toLowerCase\(\) !== "s"/);
  assert.match(main, /isTypingTarget\(event\.target as Element \| null\)/);
});

test('new copy exists in all three locales', () => {
  for (const key of [
    'settings.applyReload',
    'settings.deleteProviderBody',
    'settings.deleteAccountBody',
    'settings.deleteKeepUsage',
    'settings.searchEmpty',
    'settings.fontPacks.retry',
  ]) {
    assert.equal(count(i18n, `"${key}"`), 3, `${key} should exist in en/zh/ru`);
  }
  assert.equal(count(i18n, '"footer.updatedAgo"'), 0);
});
