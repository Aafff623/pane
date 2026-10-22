import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = ts.createSourceFile(
  'main.ts',
  readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'),
  ts.ScriptTarget.Latest,
  true,
);
const functions = source.statements.filter(
  (node) => ts.isFunctionDeclaration(node) && ['currentUiScale', 'scaledUiRect', 'uiViewport', 'placeMenuAt'].includes(node.name?.text),
);
assert.equal(functions.length, 4);

function contextWithScale(scale) {
  const document = { documentElement: {} };
  const menu = { offsetWidth: 100, offsetHeight: 50, style: {} };
  const context = vm.createContext({
    window: { innerWidth: 1000, innerHeight: 800 },
    document,
    getComputedStyle: () => ({ getPropertyValue: (name) => name === '--ui-scale' ? String(scale) : '' }),
    menu,
  });
  vm.runInContext(
    ts.transpileModule(functions.map((node) => node.getText(source)).join('\n'), {
      compilerOptions: { target: ts.ScriptTarget.ES2020 },
    }).outputText,
    context,
  );
  return { context, menu };
}

test('menu placement converts visual anchor coordinates back to layout pixels', () => {
  const { context, menu } = contextWithScale(2);
  context.placeMenuAt(menu, { left: 400, top: 100, right: 600, bottom: 300, width: 200, height: 200 }, { align: 'right' });
  assert.equal(menu.style.left, '200px');
  assert.equal(menu.style.top, '154px');
});

test('cursor menus clamp inside the zoomed viewport and flip above', () => {
  const { context, menu } = contextWithScale(2);
  context.placeMenuAt(menu, { x: 900, y: 700 }, { gap: 4 });
  assert.equal(menu.style.left, '392px');
  assert.equal(menu.style.top, '296px');
});
