import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);

function chrome(panel, pinned) {
  const fn = ast.statements.find((n) => ts.isFunctionDeclaration(n) && n.name?.text === 'popoverChromeHtml');
  const code = ts.transpileModule(fn.getText(ast), { compilerOptions: { target: ts.ScriptTarget.ES2020 } }).outputText;
  const ctx = vm.createContext({
    IS_PANEL_FORM: panel,
    config: { popoverPin: pinned },
    t: (key) => (key === 'popover.pinLabel' ? '置顶' : key),
    uiIcon: (name) => `<svg data-icon="${name}"></svg>`,
    escapeHtml: (v) => String(v),
  });
  vm.runInContext(code, ctx);
  return ctx.popoverChromeHtml();
}

test('the pin is its own chrome button, immediately left of the gear', () => {
  const html = chrome(false, true);
  const pin = html.indexOf('data-popover-pin');
  const gear = html.indexOf('data-popover-settings');
  assert.ok(pin >= 0, 'pin button missing from the header');
  assert.ok(gear > pin, 'pin must come before the gear');
  assert.match(html.slice(pin, gear), /pushPin/, 'the pin uses the push-pin icon');
  assert.match(html.slice(gear), /gear/, 'the gear keeps its own button');
});

test('the pin reports its state without needing a label', () => {
  const on = chrome(false, true);
  assert.match(on, /class="mini-btn chrome-btn pin-btn on"/);
  assert.match(on, /aria-pressed="true"/);
  const off = chrome(false, false);
  assert.match(off, /class="mini-btn chrome-btn pin-btn"/);
  assert.doesNotMatch(off, /pin-btn on/);
  assert.match(off, /aria-pressed="false"/);
  assert.match(on, /title="置顶"/, 'tooltip still says what the button does');
});

test('the panel form gets no popover chrome', () => {
  assert.equal(chrome(true, true), '');
});

test('the gear menu no longer carries a pin row and the router owns the click', () => {
  assert.doesNotMatch(source, /data-settings-pin/, 'the menu entry moved out for good');
  assert.match(source, /closest<HTMLElement>\("\[data-popover-pin\]"\)/);
  assert.doesNotMatch(source, /pin and position lock|置顶与位置锁定/,
    'the gear tooltip must not advertise a control it no longer holds');
});
