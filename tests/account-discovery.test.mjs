import test from "node:test";
import assert from "node:assert/strict";
import { fingerprintIdentity, reconcileDiscovery } from "../src/accountDiscovery.ts";

test("first sight records a baseline without announcing it", () => {
  const result = reconcileDiscovery({}, { "codex:local": "aa" });
  assert.deepEqual(result.discovered, []);
  assert.deepEqual(result.removed, []);
  assert.deepEqual(result.baseline, { "codex:local": "aa" });
});

test("a changed identity announces exactly that slot and updates the baseline", () => {
  const result = reconcileDiscovery(
    { "codex:local": "aa", "claude:local": "bb" },
    { "codex:local": "cc", "claude:local": "bb" },
  );
  assert.deepEqual(result.discovered, ["codex:local"]);
  assert.deepEqual(result.baseline, { "codex:local": "cc", "claude:local": "bb" });
});

test("a vanished identity drops its baseline; coming back counts as new", () => {
  const gone = reconcileDiscovery({ "kimi:local": "aa" }, {});
  assert.deepEqual(gone.removed, ["kimi:local"]);
  assert.deepEqual(gone.baseline, {});
  const back = reconcileDiscovery(gone.baseline, { "kimi:local": "aa" });
  assert.deepEqual(back.discovered, []);
  assert.deepEqual(back.baseline, { "kimi:local": "aa" });
});

test("fingerprints are folded and stable, never the raw identity", () => {
  const first = fingerprintIdentity(["acct-1", "oauth", "person@example.com"]);
  assert.equal(first, fingerprintIdentity(["acct-1", "oauth", "person@example.com"]));
  assert.notEqual(first, fingerprintIdentity(["acct-2", "oauth", "person@example.com"]));
  assert.ok(!first.includes("person"));
  assert.match(first, /^[0-9a-f]{8}$/);
});
