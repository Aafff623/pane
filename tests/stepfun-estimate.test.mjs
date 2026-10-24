import test from 'node:test';
import assert from 'node:assert/strict';
import { estimateStepfunCredits } from '../src/stepfunEstimate.ts';

test('Step 5 Preview credits follow the official CNY rates at a 70/30 mix', () => {
  const out = estimateStepfunCredits([{ model: 'step-5-preview', tokens: 1_000_000 }]);
  assert.equal(out.credits, 10_900_000);
  assert.equal(out.unknownTokens, 0);
});

test('vendor prefixes and flash models still match their rate rows', () => {
  const out = estimateStepfunCredits([
    { model: 'stepfun/step-3.7-flash', tokens: 1_000_000 },
    { model: 'StepFun · step-3.5-flash', tokens: 1_000_000 },
  ]);
  assert.equal(out.credits, 3_375_000 + 1_120_000);
});

test('unknown models are counted separately, never assigned an invented rate', () => {
  const out = estimateStepfunCredits([
    { model: 'step-9-unreleased', tokens: 250_000 },
    { model: 'step-5-preview', tokens: 1_000_000 },
  ]);
  assert.equal(out.credits, 10_900_000);
  assert.equal(out.unknownTokens, 250_000);
  assert.deepEqual(out.unknownModels, ['step-9-unreleased']);
});

test('zero and invalid rows contribute nothing', () => {
  const out = estimateStepfunCredits([
    { model: 'step-5-preview', tokens: 0 },
    { model: 'step-5-preview', tokens: Number.NaN },
  ]);
  assert.equal(out.credits, 0);
  assert.equal(out.unknownTokens, 0);
});
