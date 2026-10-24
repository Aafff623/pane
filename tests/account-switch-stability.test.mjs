import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);

function fn(name) {
  const node = ast.statements.find((n) => ts.isFunctionDeclaration(n) && n.name?.text === name);
  assert.ok(node, `${name} must exist`);
  return ts.transpileModule(node.getText(ast), { compilerOptions: { target: ts.ScriptTarget.ES2020 } }).outputText;
}

test('a card move becomes the right number of layout pixels at any body zoom', () => {
  const ctx = vm.createContext({ Number });
  vm.runInContext(fn('switchScrollCorrection'), ctx);
  // 1:1 at zoom 1; 3 visual px at zoom 1.5 is 2 layout px; bad zoom falls back.
  assert.equal(ctx.switchScrollCorrection(0, 0, 1), 0);
  assert.equal(ctx.switchScrollCorrection(100, 103, 1), -3);
  assert.equal(ctx.switchScrollCorrection(100, 103, 1.5), -2);
  assert.equal(ctx.switchScrollCorrection(100, 101, 0), -1);
  assert.equal(ctx.switchScrollCorrection(100, 101, NaN), -1);
});

test('the account switch keeps the clicked card in place and animates the panel', () => {
  // The anchor is captured where the click happens, and applied after the
  // scroll restore, before paint — never as a second visible jump.
  assert.match(source, /accountSwitchAnchor = \{ family, top: card\.getBoundingClientRect\(\)\.top \}/);
  assert.match(source, /switchScrollCorrection\(/);
  assert.match(source, /el\.scrollTop \+= switchScrollCorrection\(/);
  const restore = source.indexOf('el.scrollTop = scrollTop;');
  const compensate = source.indexOf('el.scrollTop += switchScrollCorrection(');
  assert.ok(restore > 0 && compensate > restore, 'compensation runs after the scroll restore');
  // One pass only: the class is gated on the family that was switched.
  assert.match(source, /animateAccountSwitchId && animateAccountSwitchId === s\.id \? " acct-switch" : ""/);
  assert.match(styles, /@keyframes acct-switch-in/);
  assert.match(styles, /\.card-panel\.acct-switch\s*\{[^}]*animation:\s*acct-switch-in\s+0\.18s/);
  // Reduced motion and the in-app toggle both take the animation away.
  assert.match(styles, /body\.reduce-anim \.card-panel\.acct-switch\s*\{[^}]*animation:\s*none/);
  assert.match(styles, /@media \(prefers-reduced-motion: reduce\)\s*\{\s*\.card-panel\.acct-switch\s*\{\s*animation:\s*none/);
});
