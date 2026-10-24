import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { primaryMeterTrend } from '../src/usageUnits.ts';
const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');

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

test('quota percent is a fallback after real quantities, never their substitute', () => {
  const percent = { id: 'minimax', unit: 'percent', trend: [66] };
  const tokens = { id: 'minimax', unit: 'tokens', trend: [null] };
  assert.equal(primaryMeterTrend('minimax', [percent, tokens]), percent);
  const credits = { id: 'minimax', unit: 'credits', trend: [12] };
  assert.equal(primaryMeterTrend('minimax', [percent, credits]), credits);
});

test('the trend layout is reconciled after this refresh has written its samples', () => {
  const fetchAt = source.indexOf('let snapshots = await invoke<Snapshot[]>("fetch_usage"');
  const meterAt = source.indexOf('await invoke<MeterTrend[]>("fetch_meter_history")');
  const applyAt = source.indexOf('lastSnapshots = snapshots;', fetchAt);
  assert.ok(fetchAt > 0, 'fetch_usage remains the refresh entry point');
  assert.ok(meterAt > fetchAt, 'meter history must be re-read after fetch_usage writes it');
  assert.ok(applyAt > meterAt, 'the fresh series must exist before the board applies its snapshots');
});

test('a percent trend shows truthfully at zero and is scaled against 100%', () => {
  assert.match(source, /v > 0 \|\| source\.unit === "percent"/, 'zero percent still earns a row');
  assert.match(source, /source\.unit === "percent" \? 100/, 'a 24% bar must not fill the chart');
  assert.match(source, /card\.quotaPeak/, 'the row says what its peak is');
});
