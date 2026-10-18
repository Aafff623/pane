import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { displayMetricLabel, setActiveLocale } from '../src/i18n.ts';

const rust = readFileSync(new URL('../src-tauri/src/i18n.rs', import.meta.url), 'utf8');
const frontend = readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
const mapping = frontend.slice(frontend.indexOf('const METRIC_KEYS:'), frontend.indexOf('\n};', frontend.indexOf('const METRIC_KEYS:')));
const mapped = new Set([...mapping.matchAll(/^\s*(?:"([^"]+)"|(\w+)):\s*"/gm)].map((m) => m[1] ?? m[2]));

test('Rust metric labels have frontend localization mappings in both shipped locales', () => {
  for (const locale of ['zh', 'ru']) {
    const start = rust.indexOf(`fn ${locale}_metric(`);
    assert.ok(start >= 0);
    const body = rust.slice(start, rust.indexOf('\n}', start));
    const labels = [...body.matchAll(/"([^"]+)"\s*=>\s*"[^"]*"\.into\(\)/g)].map((m) => m[1]);
    assert.ok(labels.length >= 30, 'Rust label contract parser found too few arms');
    setActiveLocale(locale);
    for (const label of labels) {
      assert.ok(mapped.has(label) || label === 'Reset credit', `missing frontend mapping: ${label}`);
      assert.equal(typeof displayMetricLabel(label), 'string');
    }
    assert.equal(displayMetricLabel('Unrecognized vendor label'), 'Unrecognized vendor label');
  }
  setActiveLocale('en');
});
