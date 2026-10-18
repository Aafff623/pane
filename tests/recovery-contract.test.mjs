import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { normalizeRecoveryCode, extractRecoveryCode } from '../src/recoveryCode.ts';

test('recovery input strips the Rust whitespace set without Unicode uppercase expansion', () => {
  const vectors = JSON.parse(readFileSync(new URL('./fixtures/recovery-normalization.json', import.meta.url), 'utf8'));
  for (const vector of vectors) assert.equal(normalizeRecoveryCode(vector.input), vector.normalized);
});

test('TXT extraction accepts only a whole Crockford-shaped line', () => {
  const code = '0123456789ABCDEFGHJKMNPQRSTVWXYZ01';
  assert.equal(code.length, 34);
  assert.equal(extractRecoveryCode(`Pane recovery code\n${code}\nKeep offline`), code);
  assert.equal(extractRecoveryCode('x'.repeat(33) + 'U'), null);
  assert.equal(extractRecoveryCode('1'.repeat(32) + 'ß'), null);
});
