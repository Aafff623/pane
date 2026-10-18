import test from 'node:test';
import assert from 'node:assert/strict';
import { isProviderInPeak } from '../src/peakHours.ts';

const utc = (day, hour, minute = 0) => Date.UTC(2026, 9, day, hour, minute);

test('peak windows use Beijing time and half-open boundaries', () => {
  // 2026-10-19 is Monday; UTC 07:00 is Beijing 15:00.
  assert.equal(isProviderInPeak('zai', utc(19, 7)), true);
  assert.equal(isProviderInPeak('zai', utc(19, 10)), false);
  assert.equal(isProviderInPeak('zai', utc(19, 6, 59)), true);
  assert.equal(isProviderInPeak('zai', utc(19, 10, 0)), false);
});

test('daily providers include weekends while weekday rules exclude them', () => {
  // 2026-10-18 is Sunday; UTC 04:00 is Beijing 12:00.
  assert.equal(isProviderInPeak('qodercn', utc(18, 4)), true);
  assert.equal(isProviderInPeak('traecn', utc(18, 13, 59)), true);
  assert.equal(isProviderInPeak('commandcode', utc(18, 4)), false);
  assert.equal(isProviderInPeak('unknown', utc(18, 4)), false);
});
