import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const lib = readFileSync(new URL('../src-tauri/src/lib.rs', import.meta.url), 'utf8');
const probe = readFileSync(new URL('../src-tauri/src/source_probe.rs', import.meta.url), 'utf8');

test('gear panel and HTTP probe share one credential-hint table', () => {
  // get_credential_status must read the probe's hint instead of keeping a
  // second family match that can drift from source_probe.
  assert.match(lib, /let probe = source_probe::probe_family\(&family\);/);
  assert.match(lib, /let mut local_cli = probe\.local_hint\.clone\(\);/);
  assert.doesNotMatch(lib, /"qodercn" => providers::qodercn::local_credential_hint\(\)/);

  // The shared table lives in source_probe and covers the families that the
  // HTTP API used to miss, including the presence-only ZCode check.
  assert.match(probe, /pub fn local_credential_hint\(family: &str\)/);
  for (const family of ['qodercn', 'orcaterm', 'shandianshuo', 'stepfun-plan', 'traecn', 'sensenova']) {
    assert.match(
      probe,
      new RegExp(`"${family}" => crate::providers::[a-z_]+::local_credential_hint`),
      `${family} must stay in the shared hint table`,
    );
  }
  assert.match(probe, /"zcode" => crate::providers::zcode::local_credential_hint\(\)/);
  assert.doesNotMatch(probe, /fn local_hint\(family: &str\)/);
});
