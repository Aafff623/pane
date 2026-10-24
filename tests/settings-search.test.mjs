import test from 'node:test';
import assert from 'node:assert/strict';
import {
  filterSearchEntries,
  normalizeSearchQuery,
  searchCoverage,
  stepSelection,
} from '../src/settings/search.ts';

const entries = [
  { anchor: 'Language', label: 'Language', section: 'General', page: 'general' },
  { anchor: 'Proxy%20URL', label: 'Proxy URL', section: 'Network', page: 'network' },
  { anchor: 'Refresh%20every', label: 'Refresh every', section: 'General', page: 'general' },
  { anchor: 'Font', label: 'Font', section: 'General', page: 'general' },
];

test('an empty query returns no dropdown rows', () => {
  assert.deepEqual(filterSearchEntries(entries, ''), []);
  assert.deepEqual(filterSearchEntries(entries, '   '), []);
  assert.equal(normalizeSearchQuery('  Proxy '), 'proxy');
});

test('matches label and section, case-insensitively, across pages', () => {
  assert.deepEqual(filterSearchEntries(entries, 'language').map((e) => e.anchor), ['Language']);
  assert.deepEqual(filterSearchEntries(entries, 'network').map((e) => e.anchor), ['Proxy%20URL']);
  assert.deepEqual(filterSearchEntries(entries, 'PROXY').map((e) => e.anchor), ['Proxy%20URL']);
});

test('dedupes anchors and caps the dropdown', () => {
  const duplicated = [...entries, { ...entries[0], section: 'Elsewhere' }];
  assert.equal(filterSearchEntries(duplicated, 'general').length, 3);
  assert.equal(filterSearchEntries(entries, 'general', 2).length, 2);
});

test('arrow selection wraps in both directions', () => {
  assert.equal(stepSelection(-1, 3, 1), 0);
  assert.equal(stepSelection(2, 3, 1), 0);
  assert.equal(stepSelection(0, 3, -1), 2);
  assert.equal(stepSelection(0, 0, 1), -1);
});

test('two-way coverage reports missing and orphaned anchors', () => {
  const rendered = ['Language', 'Proxy%20URL', 'Font', 'New%20row'];
  const index = entries.filter((entry) => entry.anchor !== 'Refresh%20every').concat([
    { anchor: 'Gone', label: 'Gone', section: 'General', page: 'general' },
  ]);
  const coverage = searchCoverage(index, rendered);
  assert.deepEqual(coverage.missing, ['New%20row']);
  assert.deepEqual(coverage.orphaned, ['Gone']);
  assert.deepEqual(searchCoverage(entries, entries.map((e) => e.anchor)), {
    missing: [],
    orphaned: [],
  });
});
