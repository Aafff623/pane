import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHeaderTracker } from './public/header-nav.js';

test('header stays visible at the top, hides on downward travel and reveals upward', () => {
  const track = createHeaderTracker();
  assert.equal(track(0), false);
  assert.equal(track(50), false);
  assert.equal(track(140), true);
  assert.equal(track(230), true);
  assert.equal(track(216), false);
  assert.equal(track(250), true);
  assert.equal(track(0), false);
});

test('small changes accumulate only within the same direction, avoiding flicker', () => {
  const track = createHeaderTracker(200);
  assert.equal(track(200), true);
  assert.equal(track(196), true);
  assert.equal(track(198), true);
  assert.equal(track(194), true);
  assert.equal(track(191), true);
  assert.equal(track(188), false);
  assert.equal(track(193), false);
  assert.equal(track(198), true);
});

test('restored deep positions start hidden and top overscroll restores the header', () => {
  const track = createHeaderTracker(900, 58);
  assert.equal(track(900), true);
  assert.equal(track(880), false);
  assert.equal(track(980), true);
  assert.equal(track(-20), false);
  assert.equal(track(10), false);
});
