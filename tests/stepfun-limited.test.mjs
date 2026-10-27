import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8');

test('the StepFun account form offers web login and API key with an explicit warning', () => {
  const form = source.slice(
    source.indexOf('if (id === "stepfun-plan") {'),
    source.indexOf('if (id === "sensenova") {'),
  );
  assert.ok(form.includes('data-stepfun-mode-btn="web"'));
  assert.ok(form.includes('data-stepfun-mode-btn="key"'));
  assert.ok(form.includes('data-cust-api-key="${id}"'));
  assert.ok(form.includes('customize.apiKeyApproxNote'));
  assert.ok(source.includes('customize.apiKeyModeAlert'), 'switching to a key must warn');
  assert.ok(source.includes('setStepfunCredentialMode(panel, "key")'));
  assert.ok(
    source.includes('stepfunMode === "key" ? "[data-cust-api-key]" : "[data-cust-key]"'),
    'test/save must read the field of the active mode',
  );
  assert.match(styles, /\.cred-mode-btn\.on\s*\{/);
});

test('a limited snapshot draws only the trend, its estimate, and the help button', () => {
  assert.ok(source.includes('if (shown.limited)'), 'limited cards take a dedicated branch');
  assert.ok(source.includes('limitedTrendSource(shown)'));
  assert.ok(source.includes('data-help-title="${escapeHtml(t("card.limitedHelpTitle"))}"'));
  assert.ok(source.includes('data-help-body="${escapeHtml(`${t("card.limitedHelpBody")} ${t("card.limitedHelpAction")}`)}"'));
  assert.ok(source.includes('card.limitedHelpBody'));
  assert.ok(source.includes('card.limitedNoUsage'));
  assert.ok(source.includes('source.estimated ?'), 'the trend row carries the estimated badge');
  assert.match(styles, /\.limited-help\s*\{/);
  assert.match(styles, /\.trend-estimated\s*\{/);
});
