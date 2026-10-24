import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const ast = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
const fn = (name) => ts.transpile(
  ast.statements.filter(n => ts.isFunctionDeclaration(n) && n.name?.text === name).map(n => n.getText(ast)).join('\n'),
  {target: ts.ScriptTarget.ES2020},
);

test('the untouched-window answer comes from the backend flag, not a local rule', () => {
  const ctx = vm.createContext({});
  vm.runInContext(fn('metricNotStarted'), ctx);
  assert.equal(ctx.metricNotStarted({pace: {not_started: true}}), true);
  assert.equal(ctx.metricNotStarted({pace: {not_started: false}}), false);
  // No pace at all (text rows, older snapshots) must never be silenced.
  assert.equal(ctx.metricNotStarted({}), false);
});

test('every countdown surface reads the one flag instead of re-deciding', () => {
  // The card foot used to derive the echo itself — "reset instant is a full
  // period ahead" written inline — which is why the same untouched window read
  // differently on the ring, the badge and the hover. That expression must not
  // come back anywhere outside pace.rs.
  assert.doesNotMatch(source, /notStarted = [^;\n]*resets_at[^;\n]*period_ms/,
    'a second local echo rule must not come back');
  // The remaining `"5h"` compares pick which row *is* the session window; they
  // must not be the reason a row reads as untouched. Count the real consumers:
  // card foot, nearest-reset picker, hover, ring meta, badge meta, summary
  // line, the pending-reset list and the quota builder itself.
  const consumers = (source.match(/metricNotStarted\(/g) || []).length
    + (source.match(/quota\.notStarted/g) || []).length
    + (source.match(/notStarted: metricNotStarted/g) || []).length;
  assert.ok(consumers >= 8, `expected the shared flag at every surface, saw ${consumers}`);
  assert.doesNotMatch(source, /resets_at - Date\.now\(\) >= [^;\n]*period_ms/,
    'the full-window echo test belongs to pace.rs only');
  for (const surface of ['nearestResetSeconds', 'formatHoverReset', 'extractOverviewQuota']) {
    assert.match(fn(surface), /metricNotStarted|notStarted/, `${surface} must consult the shared flag`);
  }
  // The overview ring, the collapsed badge and the summary line all branch on
  // quota.notStarted, and the live-countdown hook stays off untouched rows.
  const matches = source.match(/quota\.notStarted/g) || [];
  assert.ok(matches.length >= 3, `expected the quota flag at ring/badge/summary, saw ${matches.length}`);
  assert.match(source, /!quota\.notStarted \? ` data-reset-at=/,
    'the countdown ticker must not attach to an untouched window');
});
