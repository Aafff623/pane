import test from 'node:test';
import assert from 'node:assert/strict';
import { overviewTextHeadline } from '../src/overviewTextMetrics.ts';

test('a short balance replaces the unknown-ratio question mark', () => {
  assert.equal(overviewTextHeadline({ valueText: '¥7.13' }, 12), '¥7.13');
  assert.equal(overviewTextHeadline({ valueText: '$12.34' }, 6), '$12.34');
});

test('long or missing text keeps the existing fallback', () => {
  assert.equal(overviewTextHeadline({ valueText: 'Balance is available; team pool unknown' }, 12), null);
  assert.equal(overviewTextHeadline({ valueText: '   ' }, 12), null);
  assert.equal(overviewTextHeadline({ valueText: null }, 12), null);
});
