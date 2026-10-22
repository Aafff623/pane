import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = ts.createSourceFile(
  'skins.ts',
  readFileSync(new URL('../src/skins.ts', import.meta.url), 'utf8'),
  ts.ScriptTarget.Latest,
  true,
);
const fn = source.statements.find(
  (node) => ts.isFunctionDeclaration(node) && node.name?.text === 'embedSkinImages',
);
assert.ok(fn, 'embedSkinImages must exist');

function run(vars) {
  const context = vm.createContext({
    document: { documentElement: {} },
    getComputedStyle: () => ({ getPropertyValue: (name) => vars[name] ?? '' }),
  });
  vm.runInContext(
    ts.transpileModule(fn.getText(source).replace(/^export\s+/, ''), {
      compilerOptions: { target: ts.ScriptTarget.ES2020 },
    }).outputText,
    context,
  );
  return context.embedSkinImages;
}

const ATLAS = 'url("data:image/webp;base64,AAA/BBB+CCC=")';

test('signal atlas URLs keep the snapshot attribute well formed', async () => {
  const embed = run({ '--post-atlas': ATLAS, '--post-atlas-extra': ATLAS });
  const out = await embed('<span style="background-image:var(--post-atlas)"></span>', 'signal');
  assert.equal(out, `<span style="background-image:url('data:image/webp;base64,AAA/BBB+CCC=')"></span>`);
  assert.ok(!out.includes('url("'), 'double quotes must not leak into style attributes');
});

test('missing pack variables leave the markup untouched', async () => {
  const embed = run({});
  const markup = '<span style="background-image:var(--post-atlas)"></span>';
  assert.equal(await embed(markup, 'signal'), markup);
});

test('other skins never rewrite post-icon markup', async () => {
  const embed = run({ '--post-atlas': ATLAS });
  const markup = '<span style="background-image:var(--post-atlas)"></span>';
  assert.equal(await embed(markup, 'ink'), markup);
});
