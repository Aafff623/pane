import test from 'node:test';
import assert from 'node:assert/strict';
import { primaryMeterTrend } from '../src/usageUnits.ts';

test('speech minutes take precedence while credits remain a separate usage series', () => {
  const credits = { id: 'shandianshuo', unit: 'credits', trend: [7] };
  const minutes = { id: 'shandianshuo', unit: 'minutes', trend: [2.5] };
  assert.equal(primaryMeterTrend('shandianshuo', [credits, minutes]), minutes);
});

test('requests are an honest fallback and zero observed usage is distinct from missing data', () => {
  const requests = { id: 'provider@work', unit: 'requests', trend: [0] };
  assert.equal(primaryMeterTrend('provider@work', [
    { id: 'provider@work', unit: 'credits', trend: [null] }, requests,
  ]), requests);
  assert.equal(primaryMeterTrend('provider@other', [requests]), undefined);
  assert.equal(primaryMeterTrend('unknown', []), undefined);
});
