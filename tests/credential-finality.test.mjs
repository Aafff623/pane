import test from 'node:test';
import assert from 'node:assert/strict';
import { credentialFinality } from '../src/credentialPolicy.ts';

test('only explicit revocation is terminal', () => {
  assert.equal(credentialFinality('token_revoked'), 'revoked');
  assert.equal(credentialFinality('invalid_grant: refresh token was revoked'), 'revoked');
  assert.equal(credentialFinality('refresh_token_invalidated'), 'revoked');
});

test('generic auth, network and throttling errors stay transient', () => {
  assert.equal(credentialFinality('HTTP 401 unauthorized'), 'transient');
  assert.equal(credentialFinality('HTTP 403 forbidden'), 'transient');
  assert.equal(credentialFinality('HTTP 503 service unavailable'), 'transient');
  assert.equal(credentialFinality('timeout while contacting the proxy'), 'transient');
  assert.equal(credentialFinality('HTTP 429 retry_after_s=60'), 'transient');
});

test('key and local credential families have no revocation concept', () => {
  assert.equal(credentialFinality('token_revoked', 'api_key'), 'transient');
  assert.equal(credentialFinality('invalid_grant', 'local'), 'transient');
});
