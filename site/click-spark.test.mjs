import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createClickSpark } from './public/effects/click-spark.js';

// Exercise input deduplication and resource lifetimes with a tiny canvas host.
function fixture() {
  class Element {
    handlers = new Map(); children = []; style = {}; width = 0; height = 0;
    addEventListener(type, fn) { this.handlers.set(type, fn); }
    removeEventListener(type) { this.handlers.delete(type); }
    append(child) { child.owner = this; this.children.push(child); }
    appendChild(child) { this.append(child); }
    setAttribute() {}
    getBoundingClientRect() { return { left:100, top:200, width:180, height:42 }; }
    remove() { this.owner.children = this.owner.children.filter(child => child !== this); }
    getContext() { return { fillStyle:'#64d7c2', setTransform() {}, clearRect() {}, beginPath() {}, moveTo() {}, lineTo() {}, stroke() {} }; }
    emit(type, extra = {}) { this.handlers.get(type)?.({ target:this, clientX:160, clientY:220, button:0, ...extra }); }
  }
  const previous = Object.fromEntries(['document', 'window', 'Element', 'requestAnimationFrame', 'cancelAnimationFrame'].map(key => [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
  const frames = new Map(); let id = 0, disabled = false;
  Object.assign(globalThis, {
    Element, document:{ hidden:false, createElement:() => new Element() },
    window:{ innerWidth:1200, innerHeight:800, devicePixelRatio:2 },
    requestAnimationFrame:fn => { frames.set(++id, fn); return id; }, cancelAnimationFrame:n => frames.delete(n),
  });
  const root = new Element();
  const engine = createClickSpark(root, { isDisabled:() => disabled });
  return {
    root, engine, frames,
    get count() { return root.children[0].children.length; },
    set disabled(value) { disabled = value; },
    finish() { const callbacks = [...frames.values()]; frames.clear(); callbacks.forEach(fn => fn(performance.now() + 1500)); },
    restore() { engine.destroy(); for (const [key, descriptor] of Object.entries(previous)) { if (descriptor) Object.defineProperty(globalThis, key, descriptor); else delete globalThis[key]; } },
  };
}

test('mouse, touch and keyboard each produce one burst and expired canvases are removed', () => {
  const f = fixture();
  try {
    f.root.emit('pointerdown', { pointerType:'mouse' });
    f.root.emit('click', { pointerType:'mouse', detail:1 });
    assert.equal(f.count, 1);
    // Legacy MouseEvent after the same pointer press is also deduplicated.
    f.root.emit('click', { detail:1 }); assert.equal(f.count, 1);
    f.root.emit('pointerdown', { pointerType:'touch' });
    f.root.emit('click', { pointerType:'touch', detail:1, clientX:200, clientY:240 });
    assert.equal(f.count, 2);
    f.root.emit('click', { detail:0 }); assert.equal(f.count, 3);
    f.finish(); assert.equal(f.count, 0); assert.equal(f.frames.size, 0);
  } finally { f.restore(); }
});

test('pause blocks new bursts, clear cancels pending frames and destroy removes listeners', () => {
  const f = fixture();
  try {
    f.root.emit('click', { detail:0 }); assert.equal(f.count, 1);
    f.disabled = true; f.engine.clear();
    assert.equal(f.count, 0); assert.equal(f.frames.size, 0);
    f.root.emit('click', { detail:0 }); assert.equal(f.count, 0);
    f.disabled = false; document.hidden = true;
    f.root.emit('click', { detail:0 }); assert.equal(f.count, 0);
    f.engine.destroy(); assert.equal(f.root.handlers.size, 0);
    assert.equal(f.root.children.length, 0);
  } finally { f.restore(); }
});

test('burst canvases remain bounded under repeated clicks', () => {
  const f = fixture();
  try {
    for (let i = 0; i < 80; i++) f.root.emit('click', { detail:0 });
    assert.equal(f.count, 32);
    assert.equal(f.frames.size, 1);
  } finally { f.restore(); }
});
