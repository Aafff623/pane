import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = readFileSync(new URL('../src/resetWake.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('resetWake.ts', source, ts.ScriptTarget.Latest, true);
const badge = ast.statements.find((node) => ts.isFunctionDeclaration(node) && node.name?.text === 'resetWakeBadge');
assert.ok(badge, 'production resetWakeBadge must exist');
const code = ts.transpile(badge.getText(ast).replace(/^export /, ''), { target: ts.ScriptTarget.ES2020 });

test('reset wake badge honors enabled settings and maps backend states to localized text', () => {
  let line;
  const context = vm.createContext({
    settings: { zai: true, minimax: false },
    statuses: [],
    document: { createElement: () => (line = { title: '', outerHTML: '<p></p>' }) },
    t: (key) => key,
  });
  vm.runInContext(code, context);
  assert.equal(context.resetWakeBadge('unknown'), '');
  assert.equal(context.resetWakeBadge('minimax'), '');
  for (const status of ['success', 'failed', 'attempted', 'skipped']) {
    context.statuses = [{ provider: 'zai', status, message: 'synthetic failure' }];
    context.resetWakeBadge('zai');
    assert.equal(line.textContent, `settings.wake.${status}`);
    assert.equal(line.title.includes('synthetic failure'), status === 'failed');
  }
  for (const statuses of [[], [{ provider: 'zai', status: 'future-state' }]]) {
    context.statuses = statuses;
    context.resetWakeBadge('zai');
    assert.equal(line.textContent, 'settings.wake.waiting');
  }
});
