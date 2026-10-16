// node --test tests/key-availability.test.mjs
//
// The card-head and overview-tile badges for the search/MCP families count
// key rows by the backend `available` flag. These lock the counting rules:
// only flagged rows are keys, and fewer than two keys shows no badge.
import test from 'node:test';
import assert from 'node:assert/strict';
import { keyAvailabilityCounts, keyAvailabilityRows, keyAvailabilitySummary, overviewFoldCount } from '../src/keyAvailability.ts';

const key = (available) => ({ label: 'k', kind: 'text', value: '¥1.00', available });
const headline = () => ({ label: 'Balance', kind: 'text', value: '¥1.00', available: null });

test('fewer than two key rows shows no badge', () => {
  assert.equal(keyAvailabilitySummary(undefined), null);
  assert.equal(keyAvailabilitySummary(null), null);
  assert.equal(keyAvailabilitySummary([]), null);
  assert.equal(keyAvailabilitySummary([key(true)]), null, 'one key needs no badge');
  assert.equal(keyAvailabilitySummary([key(false)]), null);
});

test('snapshots without key rows never badge', () => {
  // Every other provider leaves `available` unset/null on all rows.
  assert.equal(keyAvailabilitySummary([headline(), { label: 'Weekly', kind: 'progress' }]), null);
});

test('headline and summary rows are excluded from the key count', () => {
  const summary = keyAvailabilitySummary([headline(), key(true), key(false), key(true)]);
  assert.deepEqual(summary, { total: 3, available: 2 });
});

test('unavailable keys count toward the total', () => {
  assert.deepEqual(keyAvailabilitySummary([key(false), key(false)]), { total: 2, available: 0 });
  assert.deepEqual(keyAvailabilitySummary([key(true), key(true)]), { total: 2, available: 2 });
});

test('counts keep the 0- and 1-row cases the badge summary drops', () => {
  // The card fold distinguishes them: 1 flagged row is a single-key card
  // (no fold control, no badge), 0 rows is not a key card at all.
  assert.deepEqual(keyAvailabilityCounts(undefined), { total: 0, available: 0 });
  assert.deepEqual(keyAvailabilityCounts([]), { total: 0, available: 0 });
  assert.deepEqual(keyAvailabilityCounts([headline()]), { total: 0, available: 0 });
  assert.deepEqual(keyAvailabilityCounts([key(false)]), { total: 1, available: 0 });
  assert.deepEqual(keyAvailabilityCounts([headline(), key(true), key(false)]), { total: 2, available: 1 });
});

test('key rows keep metric order and drop unflagged rows', () => {
  // The overview tile expands to exactly these rows, in card order.
  const rows = keyAvailabilityRows([headline(), key(true), headline(), key(false)]);
  assert.equal(rows.length, 2);
  assert.deepEqual(rows.map((r) => r.available), [true, false]);
  assert.deepEqual(keyAvailabilityRows(undefined), []);
  assert.deepEqual(keyAvailabilityRows([headline()]), []);
});

test('overview fold count takes accounts or keys, never their sum', () => {
  // Multi-account families fold snapshots; search/MCP families fold the key
  // rows of their single snapshot. Only one of the two is ever non-zero.
  assert.equal(overviewFoldCount(3, 0), 3, 'account count when there are no keys');
  assert.equal(overviewFoldCount(1, 4), 4, 'key count when the family has one snapshot');
  assert.equal(overviewFoldCount(1, 0), 1, 'single-row tiles fold nothing');
  assert.equal(overviewFoldCount(1, 1), 1, 'a lone key offers no fold control');
  assert.equal(overviewFoldCount(0, 2), 2, 'two keys always fold');
});
