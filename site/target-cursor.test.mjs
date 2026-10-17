import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const source = await readFile(new URL('./public/demo-target-cursor.js', import.meta.url), 'utf8');

function fixture({ fine = true, reduced = false, cursorPreference = null } = {}) {
  class Events {
    handlers = new Map();
    addEventListener(type, handler) {
      if (!this.handlers.has(type)) this.handlers.set(type, new Set());
      this.handlers.get(type).add(handler);
    }
    removeEventListener(type, handler) { this.handlers.get(type)?.delete(handler); }
    emit(type, event = {}) { for (const handler of [...this.handlers.get(type) ?? []]) handler(event); }
  }
  class Element extends Events {
    style = { setProperty(name, value) { this[name] = value; } };
    attributes = new Map();
    children = [];
    isConnected = true;
    textContent = '';
    constructor(kind = 'div') { super(); this.kind = kind; }
    append(child) { this.children.push(child); child.owner = this; }
    appendChild(child) { this.append(child); return child; }
    remove() { this.isConnected = false; this.owner.children = this.owner.children.filter(child => child !== this); }
    setAttribute(name, value) { this.attributes.set(name, value); }
    removeAttribute(name) { this.attributes.delete(name); }
    getAttribute(name) { return this.attributes.get(name) ?? null; }
    contains() { return true; }
    matches() { return this.disabled ?? false; }
    closest(selector) {
      if (selector.startsWith('input,')) return this.kind === 'input' ? this : null;
      return this.kind === 'button' ? this : null;
    }
    getBoundingClientRect() { return { left: 100, top: 80, width: 100, height: 40 }; }
    set innerHTML(_) { this.corners = Array.from({ length: 4 }, () => new Element()); this.dot = new Element(); this.label = new Element(); }
    querySelectorAll() { return this.corners; }
    querySelector(selector) { return selector.endsWith('__dot') ? this.dot : this.label; }
  }
  const window = new Events();
  const document = new Events();
  document.hidden = false;
  document.body = new Element(); document.head = new Element(); document.documentElement = new Element();
  document.createElement = kind => new Element(kind);
  const target = new Element('button');
  document.elementFromPoint = () => target;
  const media = [Object.assign(new Events(), { matches: fine }), Object.assign(new Events(), { matches: reduced })];
  const matchMedia = query => query.includes('pointer: fine') ? media[0] : media[1];
  window.matchMedia = matchMedia;
  const parent = {};
  let next = 0;
  let elapsed = 0;
  const frames = new Map();
  vm.runInNewContext(source, {
    window, document, parent, matchMedia, localStorage: { getItem: key => key === 'pane-demo-target-cursor' ? cursorPreference : null },
    location: { origin: 'https://pane.test' }, getComputedStyle: () => ({ borderTopLeftRadius: '6px' }),
    requestAnimationFrame: fn => { frames.set(++next, fn); return next; },
    cancelAnimationFrame: id => frames.delete(id),
  });
  return {
    window, document, media, target, frames,
    get root() { return document.body.children[0]; },
    move(element = target, x = 150, y = 100) { window.emit('pointermove', { pointerType: 'mouse', clientX: x, clientY: y, target: element }); },
    pause(value, overrides = {}) { window.emit('message', { source: parent, origin: 'https://pane.test', data: { type: 'pane-demo-motion', paused: value }, ...overrides }); },
    paint(dt = 16, inspect = () => {}) { for (let i = 1; i <= 90; i++) { elapsed += dt; const tasks = [...frames.values()]; frames.clear(); tasks.forEach(fn => fn(elapsed)); inspect(); } },
  };
}

test('reticle locks onto a button without capturing input, then restores native input cursors', () => {
  const f = fixture();
  f.move(); f.paint();
  assert.ok(+f.root.style.opacity > 0.99);
  assert.match(f.document.head.children[0].textContent, /cursor: none/);
  assert.ok(f.root.corners.every(corner => !corner.style.transform.includes('NaN')));
  // Top-left corner approaches target origin minus the 4px padding.
  assert.match(f.root.corners[0].style.transform, /translate3d\(96\.00px, 76\.00px/);
  f.move({ closest: () => ({}) });
  assert.equal(f.document.head.children[0].textContent, '');
});

test('shared pause validates the host and releases animation, cursor style and listeners', () => {
  const f = fixture();
  f.move();
  f.pause(true, { origin: 'https://untrusted.test' }); assert.ok(f.root);
  f.pause(true, { source: {} }); assert.ok(f.root);
  f.pause(true);
  assert.equal(f.root, undefined);
  assert.equal(f.frames.size, 0);
  assert.equal(f.document.head.children.length, 0);
  assert.equal(f.window.handlers.get('pointermove').size, 0);
  f.pause(false); f.pause(false);
  assert.equal(f.document.body.children.length, 1);
});

test('target cursor defaults on and honours an opt-out before the frame starts', () => {
  assert.ok(fixture().root);
  assert.ok(fixture({ cursorPreference: 'on' }).root);
  const f = fixture({ cursorPreference: 'off' });
  assert.equal(f.root, undefined);
  assert.equal(f.frames.size, 0);
  f.pause(false);
  assert.ok(f.root);
  f.pause(true);
  assert.equal(f.root, undefined);
  assert.equal(f.frames.size, 0);
});

test('coarse pointers, reduced motion and hidden documents retain native cursors', () => {
  assert.equal(fixture({ fine: false }).root, undefined);
  assert.equal(fixture({ reduced: true }).root, undefined);
  const f = fixture(); f.move();
  f.document.hidden = true; f.document.emit('visibilitychange');
  assert.equal(f.root, undefined); assert.equal(f.frames.size, 0);
  f.document.hidden = false; f.document.emit('visibilitychange');
  assert.ok(f.root);
  f.media[1].matches = true; f.media[1].emit('change');
  assert.equal(f.root, undefined);
});

test('leaving the frame or cancelling a pointer restores the native cursor immediately', () => {
  const f = fixture();
  for (const type of ['blur', 'pointercancel']) {
    f.move(); f.window.emit(type);
    assert.equal(f.document.head.children[0].textContent, '');
  }
  f.move(); f.document.documentElement.emit('pointerleave');
  assert.equal(f.document.head.children[0].textContent, '');
});

test('slow 50ms frames keep the roaming spring finite and within the viewport', () => {
  const f = fixture();
  const blank = { closest: () => null };
  f.move(blank); f.paint(50);
  f.move(blank, 600, 400);
  f.paint(50, () => {
    for (const corner of f.root.corners) {
      const values = [...corner.style.transform.matchAll(/-?\d+(?:\.\d+)?/g)].map(match => Number(match[0]));
      assert.ok(values.every(value => Number.isFinite(value) && Math.abs(value) < 1000), corner.style.transform);
    }
  });
});
