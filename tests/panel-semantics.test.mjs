import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const main = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const i18n = readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
const rust = readFileSync(new URL('../src-tauri/src/lib.rs', import.meta.url), 'utf8');
const count = (haystack, needle) => haystack.split(needle).length - 1;

test('quota percentages keep one labelled convention', () => {
  assert.match(main, /function showsUsed\(\): boolean/);
  assert.match(main, /return config\.meterMode !== "left";/);
  assert.match(main, /meterMode: "used",/);
  assert.match(main, /function quotaPctLabel\(usedPercent: number \| null\): string/);
  assert.match(main, /style="width:\$\{showsUsed\(\) \? used : left\}%"/);
  assert.match(main, /const pctLabel = pct === null \? null : quotaPctLabel\(pct\);/);
  assert.match(main, /fold-main-quota">\$\{escapeHtml\(headLabel/);
  assert.match(main, /settings\.meterMode/);
  assert.doesNotMatch(main, /showUsed/);
  assert.doesNotMatch(main, /Math\.round\(headPct\)/);
  // The backend drops unknown config keys; without these the switch reverts.
  assert.match(rust, /obj\.entry\("meterMode"\)\.or_insert\(json!\("used"\)\)/);
  assert.match(rust, /"meterMode",/);
  assert.doesNotMatch(rust, /"showUsed",/);
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
    'settings.meterMode',
    'settings.meterUsed',
    'settings.meterLeft',
  ]) {
    assert.equal(count(i18n, `"${key}"`), 3, `${key} should exist in en/zh/ru`);
  }
  assert.equal(count(i18n, '"footer.updatedAgo"'), 0);
});
