import test from 'node:test';
import assert from 'node:assert/strict';
import { skinTransitionKind, transitionBudget, transitionOrigin } from '../src/skinTransitionPlan.ts';

test('only the three official packs own a themed transition', () => {
  assert.equal(skinTransitionKind('comic-pixel'), 'pixel');
  assert.equal(skinTransitionKind('signal'), 'stamp');
  assert.equal(skinTransitionKind('ink'), 'ink');
  assert.equal(skinTransitionKind('native'), null);
  assert.equal(skinTransitionKind('some-future-pack'), null);
  assert.equal(skinTransitionKind(undefined), null);
});

test('a hidden or zero-size source falls back to the viewport centre', () => {
  assert.deepEqual(transitionOrigin(null, { width: 400, height: 700 }), { x: 200, y: 350 });
  assert.deepEqual(
    transitionOrigin({ left: 0, top: 0, width: 0, height: 0 }, { width: 400, height: 700 }),
    { x: 200, y: 350 },
  );
  assert.deepEqual(
    transitionOrigin({ left: 100, top: 40, width: 30, height: 20 }, { width: 400, height: 700 }),
    { x: 115, y: 50 },
  );
});

test('every timeline stays inside the 350-650ms interaction budget', () => {
  for (const skin of ['comic-pixel', 'signal', 'ink']) {
    const budget = transitionBudget(skinTransitionKind(skin));
    const total = budget.cover + budget.hold + budget.release;
    assert.ok(total >= 350 && total <= 650, `${skin}: ${total}ms`);
  }
});

import { isNewerVersion } from '../src/skins.ts';

test('a local pack newer than the deployed index is never a downgrade target', () => {
  assert.equal(isNewerVersion('1.0.1', '1.0.2'), false);
  assert.equal(isNewerVersion('1.0.2', '1.0.1'), true);
  assert.equal(isNewerVersion('1.0.1', '1.0.1'), false);
  assert.equal(isNewerVersion('1.10.0', '1.9.9'), true);
  assert.equal(isNewerVersion('2.0.0-rc.1', '1.9.0'), true);
});
