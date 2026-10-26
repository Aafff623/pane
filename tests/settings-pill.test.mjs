import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

// Extract the pill helpers from main.ts and exercise them against a DOM
// stub: the hidden select must stay the only value source, and a pill click
// must set select.value and fire a real change event (every save path hangs
// off that event).
const main = ts.createSourceFile(
  'main.ts',
  readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'),
  ts.ScriptTarget.ES2020,
  true,
);
const wanted = ['syncSelectPill', 'positionTabThumb', 'mountSelectPill'];
const code = ts.transpile(
  main.statements
    .filter((node) => ts.isFunctionDeclaration(node) && wanted.includes(node.name?.text))
    .map((node) => node.getText(main))
    .join('\n'),
  { target: ts.ScriptTarget.ES2020 },
);

function fakeElement(tag) {
  const listeners = new Map();
  const children = [];
  const el = {
    tagName: tag.toUpperCase(),
    className: '',
    type: '',
    textContent: '',
    hidden: false,
    offsetWidth: 10,
    offsetLeft: 0,
    style: {},
    dataset: {},
    attrs: {},
    children,
    listeners,
    setAttribute(k, v) { this.attrs[k] = v; },
    getAttribute(k) { return this.attrs[k] ?? null; },
    removeAttribute(k) { delete this.attrs[k]; },
    addEventListener(ev, fn) { listeners.set(ev, fn); },
    dispatchEvent(ev) { listeners.get(ev.type)?.(); return true; },
    append(child) { children.push(child); },
    querySelector(selector) {
      if (selector === ':scope > .tab-thumb') {
        return children.find((child) => child.className === 'tab-thumb') ?? null;
      }
      if (selector === ':scope > .tab.active') {
        return children.find((child) => child.className === 'tab active') ?? null;
      }
      return null;
    },
    querySelectorAll(selector) {
      return children.filter((child) =>
        selector === '.tab' ? child.className === 'tab' : false);
    },
    after(node) {
      this.insertedAfter = node;
      // The real DOM finds the pill via the parent element; mirror that so
      // syncSelectPill sees the group right after it is inserted.
      if (this.parentElement) this.parentElement.querySelector = () => node;
    },
    classList: {
      states: new Set(),
      toggle(name, on) { on ? this.states.add(name) : this.states.delete(name); },
    },
  };
  return el;
}

function fakeSelect(options, value) {
  const select = fakeElement('select');
  select.id = 'test-select';
  select.options = options.map(([value2, text]) => {
    const option = fakeElement('option');
    option.value = value2;
    option.textContent = text;
    return option;
  });
  select.value = value;
  const host = fakeElement('div');
  host.querySelector = () => null;
  select.parentElement = host;
  return select;
}

function setup() {
  const context = vm.createContext({
    document: { createElement: fakeElement },
    Event: class Event { constructor(type) { this.type = type; } },
  });
  vm.runInContext(code, context);
  return context;
}

test('mounts a pill per option and hides the select as the value source', () => {
  const context = setup();
  const select = fakeSelect([['a', 'A'], ['b', 'B']], 'a');
  context.mountSelectPill(select);
  const pill = select.insertedAfter;
  assert.ok(pill, 'the pill group is inserted after the select');
  assert.equal(pill.className, 'tabs');
  assert.equal(pill.attrs.role, 'group');
  assert.ok(pill.attrs['aria-label'], 'the pill carries the group label');
  assert.equal(pill.children.length, 3, 'thumb + two segments');
  assert.equal(pill.children[0].className, 'tab-thumb');
  assert.equal(select.hidden, true, 'the select stays in the DOM but hidden');
  assert.ok(pill.children[1].classList.states.has('active'), 'current value is active');
  assert.equal(pill.children[1].attrs['aria-pressed'], 'true');
  assert.equal(pill.children[2].attrs['aria-pressed'], 'false');
});

test('a pill click sets select.value and fires a real change event', () => {
  const context = setup();
  const select = fakeSelect([['a', 'A'], ['b', 'B']], 'a');
  let changeEvents = 0;
  select.addEventListener('change', () => { changeEvents += 1; });
  context.mountSelectPill(select);
  const pill = select.insertedAfter;
  pill.children[2].listeners.get('click')();
  assert.equal(select.value, 'b', 'the hidden select is the value source');
  assert.equal(changeEvents, 1, 'exactly one change event reaches the listeners');
  assert.ok(pill.children[2].classList.states.has('active'));
  assert.equal(pill.children[1].attrs['aria-pressed'], 'false');
  // Clicking the already-active segment is a no-op.
  pill.children[2].listeners.get('click')();
  assert.equal(changeEvents, 1);
});

test('syncSelectPill follows programmatic setSelect writes', () => {
  const context = setup();
  const select = fakeSelect([['a', 'A'], ['b', 'B']], 'a');
  context.mountSelectPill(select);
  const pill = select.insertedAfter;
  // setSelect writes select.value directly, then calls syncSelectPill.
  select.value = 'b';
  context.syncSelectPill(select);
  assert.ok(pill.children[2].classList.states.has('active'));
  assert.equal(pill.children[1].attrs['aria-pressed'], 'false');
});

test('mounting twice never stacks a second pill', () => {
  const context = setup();
  const select = fakeSelect([['a', 'A'], ['b', 'B']], 'a');
  context.mountSelectPill(select);
  context.mountSelectPill(select);
  assert.equal(select.insertedAfter.children.length, 3, 'still one group, thumb + two segments');
});
