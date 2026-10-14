import test from 'node:test';
import assert from 'node:assert/strict';
import { initialTourPending } from '../src/onboardingState.ts';

const fresh = { tourSeen: false, tourPending: false, lastSeenVersion: '', welcomeDismissed: false };
test('fresh install waits for first visible presentation', () => {
  assert.equal(initialTourPending(fresh), true);
});
test('hidden first boot survives a version-marker write and restart', () => {
  assert.equal(initialTourPending({ ...fresh, tourPending: true, lastSeenVersion: '0.8.2' }), true);
});
test('existing installs are not interrupted', () => {
  assert.equal(initialTourPending({ ...fresh, lastSeenVersion: '0.8.1' }), false);
  assert.equal(initialTourPending({ ...fresh, welcomeDismissed: true }), false);
});
test('skip and completion take precedence over a pending marker', () => {
  assert.equal(initialTourPending({ ...fresh, tourSeen: true, tourPending: true }), false);
});
