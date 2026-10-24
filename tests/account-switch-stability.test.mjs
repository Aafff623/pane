import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8');

test('an account tab swaps only its own card, never the whole board', () => {
  // Full renders let the quota overview above the card reflow by a few pixels;
  // near the list bottom the scroller is already clamped, so the card slid down
  // on every click. The tab path must rebuild just the clicked card.
  assert.ok(source.includes('function switchAccountCard(family: string, acctId: string, tab: HTMLElement): void'));
  assert.ok(source.includes('card.replaceWith(next)'));
  assert.ok(source.includes('switchAccountCard(family, acctId, acctTab);'));

  const clickBranch = source.slice(
    source.indexOf('const acctTab = target.closest<HTMLElement>("[data-card-account]")'),
    source.indexOf('const cardRefresh = target.closest<HTMLElement>("[data-card-refresh]")'),
  );
  assert.doesNotMatch(clickBranch, /requestRender\(\)/, 'the tab click must not repaint the dashboard');

  // The panel still animates the switch.
  assert.ok(source.includes('next.querySelector(".card-panel")?.classList.add("acct-switch")'));
  assert.ok(styles.includes('@keyframes acct-switch-in'));
  assert.match(styles, /\.card-panel\.acct-switch\s*\{[^}]*animation:\s*acct-switch-in\s+0\.18s/);
  assert.match(styles, /body\.reduce-anim \.card-panel\.acct-switch\s*\{[^}]*animation:\s*none/);
  assert.match(styles, /@media \(prefers-reduced-motion: reduce\)\s*\{\s*\.card-panel\.acct-switch\s*\{\s*animation:\s*none/);
});

test('the fallback still rebuilds when a card has no anchor snapshot', () => {
  const fn = source.slice(
    source.indexOf('function switchAccountCard('),
    source.indexOf('function orderedSnapshots'),
  );
  assert.ok(fn.includes('!scroller || !card || !scroller.contains(card) || !anchor'));
  assert.ok(fn.includes('requestRender();'));
});

test('a shorter swapped card restores the clamped scroll range with one spacer', () => {
  const fn = source.slice(
    source.indexOf('function switchAccountCard('),
    source.indexOf('function orderedSnapshots'),
  );
  assert.ok(fn.includes('card.replaceWith(next)'));
  assert.ok(fn.includes('scrollBefore > max + 0.5'), 'only a clamped offset needs the spacer');
  assert.ok(fn.includes('spacer.style.height = `${Math.ceil(scrollBefore - max)}px`'));
  assert.ok(
    fn.includes('scroller.querySelectorAll<HTMLElement>(":scope > .acct-switch-spacer").forEach((el) => el.remove())'),
    'the previous spacer must not stack up across clicks',
  );
  assert.match(styles, /\.acct-switch-spacer\s*\{[^}]*flex:\s*none/);
});
