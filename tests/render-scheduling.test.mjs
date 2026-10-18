import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import { RenderScheduler } from '../src/renderScheduler.ts';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
const requestRender = ast.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'requestRender');
assert.ok(requestRender);

test('a direct dashboard render does not rebuild a hidden window', () => {
  let writes = 0;
  const context = vm.createContext({
    renderScheduler: new RenderScheduler(() => { writes += 1; }, () => false, () => 1, () => {}),
  });
  vm.runInContext(ts.transpileModule(requestRender.getText(ast), {
    compilerOptions: { target: ts.ScriptTarget.ES2022 },
  }).outputText, context);
  context.requestRender();
  assert.equal(writes, 0);
});

function renderer() {
  const host = { visible: true, paints: 0, frames: new Map(), nextFrame: 0 };
  host.scheduler = new RenderScheduler(
    () => { host.paints += 1; },
    () => host.visible,
    callback => { const id = ++host.nextFrame; host.frames.set(id, callback); return id; },
    frame => host.frames.delete(frame),
  );
  host.advanceFrame = () => {
    const callbacks = [...host.frames.values()];
    host.frames.clear();
    callbacks.forEach(callback => callback());
  };
  return host;
}

test('several dashboard changes rebuild only once on the next visible frame', () => {
  const host = renderer();
  host.scheduler.request();
  host.scheduler.request();
  host.scheduler.request();
  assert.equal(host.paints, 0);
  assert.equal(host.frames.size, 1);
  host.advanceFrame();
  assert.equal(host.paints, 1);
  assert.equal(host.scheduler.pending, false);
});

test('hidden changes remain pending until the window becomes visible', () => {
  const host = renderer();
  host.visible = false;
  host.scheduler.request();
  host.scheduler.request();
  host.scheduler.flush();
  assert.equal(host.paints, 0);
  assert.equal(host.frames.size, 0);
  assert.equal(host.scheduler.pending, true);
  host.visible = true;
  host.scheduler.flush();
  assert.equal(host.paints, 1);
  assert.equal(host.scheduler.pending, false);
});

test('hiding after a render request suspends the paint instead of dropping it', () => {
  const host = renderer();
  host.scheduler.request();
  host.visible = false;
  host.advanceFrame();
  assert.equal(host.paints, 0);
  assert.equal(host.scheduler.pending, true);
  host.visible = true;
  host.scheduler.request();
  host.advanceFrame();
  assert.equal(host.paints, 1);
});

test('synchronous navigation can flush its DOM before querying it without a duplicate frame', () => {
  const host = renderer();
  const flushRender = ast.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'flushRender');
  assert.ok(flushRender);
  const context = vm.createContext({ renderScheduler: host.scheduler });
  vm.runInContext(ts.transpileModule(flushRender.getText(ast), {
    compilerOptions: { target: ts.ScriptTarget.ES2022 },
  }).outputText, context);
  context.flushRender();
  assert.equal(host.paints, 1);
  assert.equal(host.frames.size, 0);
  host.advanceFrame();
  assert.equal(host.paints, 1);
});
