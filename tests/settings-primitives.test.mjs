import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderSection } from '../src/settings/section.ts';
import { renderItem } from '../src/settings/item.ts';

const styles = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8');

test('renderSection builds the tab header and one card around the children', () => {
  const html = renderSection({
    id: 'set-general',
    title: 'General',
    desc: 'Everyday settings',
    children: '<div class="control"></div>',
  });
  assert.match(html, /<section id="set-general" class="set-section">/);
  assert.match(html, /<h2 class="set-section__title">General<\/h2>/);
  assert.match(html, /<span class="set-section__desc">Everyday settings<\/span>/);
  assert.match(html, /<div class="set-section__card"><div class="control"><\/div><\/div>/);
});

test('renderItem encodes an anchor, keeps the control HTML, and supports stacked', () => {
  const normal = renderItem({
    label: 'Refresh every',
    desc: '5 minutes',
    control: '<input id="interval" />',
  });
  assert.match(normal, /data-setting-anchor="Refresh%20every"/);
  assert.match(normal, /class="set-row"/);
  assert.match(normal, /<span class="set-row__label">Refresh every<\/span>/);
  assert.match(normal, /<span class="set-row__desc">5 minutes<\/span>/);
  assert.match(normal, /<div class="set-row__control"><input id="interval" \/><\/div>/);

  const stacked = renderItem({ label: 'Proxy URL', control: '<input />', stacked: true });
  assert.match(stacked, /class="set-row set-row--stacked"/);
});

test('an explicit anchor wins and text is escaped', () => {
  const html = renderItem({
    label: 'A & B',
    anchor: 'unique-anchor',
    control: '<button>Save</button>',
  });
  assert.match(html, /data-setting-anchor="unique-anchor"/);
  assert.match(html, /A &amp; B/);
  assert.doesNotMatch(html, /<span class="set-row__label">A & B/);
});

test('i18n keys survive on the label and description for locale switches', () => {
  const html = renderItem({
    label: 'Language',
    labelKey: 'settings.language',
    desc: 'Interface language',
    descKey: 'settings.desc.language',
    control: '<select></select>',
  });
  assert.match(html, /<span class="set-row__label" data-i18n="settings.language">Language<\/span>/);
  assert.match(html, /<span class="set-row__desc" data-i18n="settings.desc.language">Interface language<\/span>/);
  const section = renderSection({
    title: 'General',
    titleKey: 'settings.general',
    descKey: 'panel.desc.general',
    children: '',
  });
  assert.match(section, /<h2 class="set-section__title" data-i18n="settings.general">General<\/h2>/);
  assert.match(section, /<span class="set-section__desc" data-i18n="panel.desc.general"><\/span>/);
});

test('the skeleton CSS fixes the three text levels and the last-row border', () => {
  assert.match(styles, /\.set-section__title\s*\{[^}]*font-size:\s*18px/);
  assert.match(styles, /\.set-row__label\s*\{[^}]*font-size:\s*14px/);
  assert.match(styles, /\.set-row__desc\s*\{[^}]*font-size:\s*12px/);
  assert.match(styles, /\.set-section__card > \.setting-row:last-child\s*\{[^}]*border-bottom:\s*0/);
  assert.match(styles, /\.set-section__head\s*\{[^}]*margin-bottom:\s*-1px/);
});
