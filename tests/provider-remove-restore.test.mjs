import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
const names = ['removeProviderCard', 'restoreProviderFamily'];
const functions = ast.statements.filter(
  (node) => ts.isFunctionDeclaration(node) && names.includes(node.name?.text),
);
assert.equal(functions.length, names.length);
const code = ts.transpile(
  functions.map((node) => node.getText(ast)).join('\n'),
  { target: ts.ScriptTarget.ES2020 },
);

function harness() {
  const calls = { patch: [], renders: 0, drawer: 0 };
  const context = vm.createContext({
    config: {
      disabled: [],
      removedProviders: [],
      layout: {
        providerOrder: ['apigoto', 'claude'],
        providers: { apigoto: { hidden: [] }, claude: { hidden: [] } },
      },
    },
    lastSnapshots: [{ id: 'apigoto' }, { id: 'claude' }],
    custConfigOpen: null,
    providerFamily: (id) => String(id).split('@')[0],
    patchConfig: async (patch) => {
      calls.patch.push(structuredClone(patch));
      Object.assign(context.config, patch);
    },
    requestRender: () => {
      calls.renders += 1;
    },
    renderDrawerBody: () => {
      calls.drawer += 1;
    },
  });
  vm.runInContext(code, context);
  return { context, calls };
}

test('provider delete writes both lists; restore clears both', async () => {
  const { context, calls } = harness();

  context.removeProviderCard('apigoto');
  assert.deepEqual([...context.config.removedProviders], ['apigoto']);
  assert.deepEqual([...context.config.disabled], ['apigoto']);
  assert.ok(!context.config.layout.providerOrder.includes('apigoto'));
  assert.ok(!('apigoto' in context.config.layout.providers));
  assert.ok(!context.lastSnapshots.some((snapshot) => snapshot.id === 'apigoto'));

  await context.restoreProviderFamily('apigoto');
  assert.deepEqual([...context.config.removedProviders], []);
  assert.deepEqual([...context.config.disabled], []);
  assert.deepEqual([...calls.patch.at(-1).removedProviders], []);
  assert.deepEqual([...calls.patch.at(-1).disabled], []);
  assert.ok(calls.renders > 0);
  assert.ok(calls.drawer > 0);
});

test('restore clears only the target family', async () => {
  const { context } = harness();
  context.config.removedProviders = ['apigoto', 'kimi'];
  context.config.disabled = ['apigoto', 'kimi', 'claude'];

  await context.restoreProviderFamily('apigoto');
  assert.deepEqual([...context.config.removedProviders], ['kimi']);
  assert.deepEqual([...context.config.disabled], ['kimi', 'claude']);
});
