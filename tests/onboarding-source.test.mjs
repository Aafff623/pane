import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

// Exercise the detection-step helpers against a fake probe report. The real
// command is read-only; these assertions lock the evidence/status contract and
// ensure no environment values can leak into the checklist.
const main = ts.createSourceFile(
  'main.ts',
  readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'),
  ts.ScriptTarget.ES2020,
  true,
);
const names = new Set([
  'defaultOnboardingPicks',
  'seedOnboardingPicks',
  'onboardingSourceStatus',
  'onboardingSourceEvidence',
  'onboardingProbeRows',
  'onboardingSourceRowsHtml',
  'detectedFamilies',
  'picksWithEnabled',
]);
const code = ts.transpile(
  main.statements
    .filter((node) => ts.isFunctionDeclaration(node) && names.has(node.name?.text))
    .map((node) => node.getText(main))
    .join('\n'),
  { target: ts.ScriptTarget.ES2020 },
);

const dict = {
  'onboarding.scanning': 'Scanning…',
  'onboarding.detectFailed': 'Detection failed',
  'onboarding.rescan': 'Rescan',
  'onboarding.none': 'Nothing detected',
  'onboarding.foundSources': 'Detected {n} source(s)',
  'onboarding.sourceReady': 'Credential found',
  'onboarding.sourceInstalledSignedOut': 'Installed · not signed in',
  'onboarding.sourceMissing': 'Not installed',
  'onboarding.sourceUnknown': 'Install state unknown',
  'onboarding.envFound': 'Environment keys found',
  'onboarding.envUnsupported': 'Quota reading is not supported yet.',
  'onboarding.rescanHint': 'Re-reads local evidence.',
};

function makeContext(report, selection = new Set()) {
  const context = vm.createContext({
    sourceProbeReport: report,
    sourceProbeLoading: false,
    sourceProbeError: null,
    lastSnapshots: [],
    config: { disabled: ['disabled-fam'] },
    providerFamily: (id) => id.split('@')[0],
    onboardingSelection: selection,
    onboardingSelectionTouched: false,
    STARTER_FAMILIES: new Set(['claude', 'codex']),
    ALL_PROVIDERS: [['claude', 'Claude'], ['codex', 'Codex'], ['brave', 'Brave Search'], ['gemini', 'Gemini'], ['disabled-fam', 'Disabled']],
    t: (key, vars = {}) => {
      let text = dict[key] ?? key;
      for (const [name, value] of Object.entries(vars)) text = text.split(`{${name}}`).join(String(value));
      return text;
    },
    escapeHtml: (value) => String(value).replace(/[&<>"']/g, (char) => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
    }[char])),
    uiIcon: () => '',
  });
  vm.runInContext(code, context);
  return context;
}

const report = {
  families: [
    {
      family: 'claude', displayName: 'Claude', installState: 'installed',
      installEvidence: ['C:\\Users\\me\\.claude'], credentialState: 'absent',
      credentialSource: 'none', envVars: [], localHint: null,
    },
    {
      family: 'brave', displayName: 'Brave Search', installState: 'missing',
      installEvidence: [], credentialState: 'present',
      credentialSource: 'env:BRAVE_SEARCH_API_KEY', envVars: ['BRAVE_SEARCH_API_KEY'], localHint: null,
    },
  ],
  envKeys: [{ name: 'AZURE_OPENAI_API_KEY', family: null, supported: false }],
};

test('detection rows show install/credential evidence without secret values', () => {
  const ctx = makeContext(report, new Set(['claude']));
  const html = ctx.onboardingSourceRowsHtml();
  assert.match(html, /data-onboarding-pick="claude"/);
  assert.match(html, /Installed · not signed in/);
  assert.match(html, /C:\\Users\\me\\\.claude/);
  assert.match(html, /Credential found/);
  assert.match(html, /env: BRAVE_SEARCH_API_KEY/);
  assert.match(html, /AZURE_OPENAI_API_KEY/);
  assert.match(html, /Quota reading is not supported yet/);
  assert.ok(!html.includes('test-109'), 'env values must never enter the checklist');
});

test('starter picks only apply when nothing is detected', () => {
  const ctx = makeContext({ families: [], envKeys: [] });
  ctx.seedOnboardingPicks();
  assert.deepEqual([...ctx.onboardingSelection].sort(), ['claude', 'codex']);
  const real = makeContext(report);
  real.seedOnboardingPicks();
  assert.ok(real.onboardingSelection.has('claude'));
  assert.ok(real.onboardingSelection.has('brave'));
});

test('account cards count as detection for their family', () => {
  const ctx = makeContext({ families: [], envKeys: [] });
  const found = ctx.detectedFamilies([
    { id: 'apigoto', status: 'no_credentials' },
    { id: 'apigoto@abc', status: 'ok' },
    { id: 'tokenrhythm@xyz', status: 'ok' },
    { id: 'grok', status: 'no_credentials' },
  ]);
  assert.ok(found.has('apigoto'), 'family card no_credentials must not hide a working account');
  assert.ok(found.has('tokenrhythm'));
  assert.ok(!found.has('grok'));
});

test('non-fresh pick application keeps already enabled providers', () => {
  const ctx = makeContext({ families: [], envKeys: [] });
  const kept = ctx.picksWithEnabled(new Set(['chosen']), false);
  assert.ok(kept.has('chosen'));
  assert.ok(kept.has('claude'), 'enabled family must stay enabled');
  assert.ok(!kept.has('disabled-fam'), 'a deliberately disabled family stays off');
  const fresh = ctx.picksWithEnabled(new Set(['chosen']), true);
  assert.deepEqual([...fresh], ['chosen']);
});
