import test from "node:test";
import assert from "node:assert/strict";
import { createPaneApi, handleMessage, requestJson, validateBaseUrl, TOOLS } from "../mcp/pane-mcp/lib.mjs";

const fetchedAt = new Date(Date.now() - 12_000).toISOString();
const providers = {
  providers: [
    {
      providerId: "claude",
      displayName: "Claude",
      plan: "Pro",
      status: "ok",
      stale: false,
      fetchedAt,
      email: "person@example.com",
      maskedKey: "sk-live-secret",
      lines: [
        { type: "progress", label: "Session", limit: 100, used: 20, resetsAt: "2026-10-23T12:00:00Z" },
        { type: "text", label: "Extra usage", value: "$0.00" },
      ],
    },
  ],
  disabled: ["openai-api"],
};

function mockApi(overrides = {}) {
  const calls = [];
  const api = {
    calls,
    listProviders: async () => providers,
    getUsage: async (id) => ({ ...providers.providers[0], providerId: id }),
    getStatus: async () => ({ status: { version: "0.8.9" }, doctor: [] }),
    listSources: async () => ({ sources: [] }),
    listResets: async () => ({ supported: true, resets: [] }),
    ...overrides,
  };
  return api;
}

test("initialize and tools/list expose the read-only tool set", async () => {
  const init = await handleMessage({ jsonrpc: "2.0", id: 1, method: "initialize", params: {} }, mockApi());
  assert.equal(init.result.protocolVersion, "2025-06-18");
  assert.equal(init.result.capabilities.tools !== undefined, true);
  const list = await handleMessage({ jsonrpc: "2.0", id: 2, method: "tools/list" }, mockApi());
  assert.deepEqual(list.result.tools.map((t) => t.name), TOOLS.map((t) => t.name));
});

test("get_usage returns asOf/ageSecs and does not leak PII or credentials", async () => {
  const api = createPaneApi({
    baseUrl: "http://127.0.0.1:6736",
    fetchImpl: async () => ({
      ok: true,
      headers: { get: () => null },
      text: async () => JSON.stringify({
        providers: [{
          ...providers.providers[0],
          error: "Bearer sk-live-secret for person@example.com",
          warning: "https://example.test/?token=secret",
        }],
      }),
    }),
  });
  const response = await handleMessage({
    jsonrpc: "2.0", id: 3, method: "tools/call",
    params: { name: "get_usage", arguments: { provider_id: "claude" } },
  }, api);
  const text = response.result.content[0].text;
  const payload = JSON.parse(text);
  assert.equal(payload.asOf, new Date(fetchedAt).toISOString());
  assert.equal(typeof payload.ageSecs, "number");
  assert.equal(text.includes("person@example.com"), false);
  assert.equal(text.includes("sk-live-secret"), false);
  assert.equal(text.includes("secret"), false);
  assert.equal(text.includes("maskedKey"), false);
  assert.equal(text.includes("email"), false);
});

test("unknown tool is returned as a tool error, not a protocol crash", async () => {
  const response = await handleMessage({
    jsonrpc: "2.0", id: 4, method: "tools/call",
    params: { name: "refresh_provider", arguments: {} },
  }, mockApi());
  assert.equal(response.result.isError, true);
  assert.match(response.result.content[0].text, /unknown tool/);
});

test("requestJson is GET-only, bounded to loopback and rejects non-loopback bases", async () => {
  const calls = [];
  await requestJson("/v1/status", {
    baseUrl: "http://127.0.0.1:6736",
    fetchImpl: async (url, options) => {
      calls.push({ url, options });
      return { ok: true, headers: { get: () => null }, text: async () => "{\"ok\":true}" };
    },
  });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].options.method, "GET");
  assert.equal(calls[0].options.body, undefined);
  assert.throws(() => validateBaseUrl("https://example.com"), /loopback|HTTP/);
  assert.throws(() => validateBaseUrl("http://127.0.0.1:6736/v1"), /path/);
});

test("an unreachable Pane API returns a clear tool error", async () => {
  const api = createPaneApi({
    baseUrl: "http://127.0.0.1:1",
    fetchImpl: async () => { throw new TypeError("fetch failed"); },
    now: () => Date.now(),
  });
  const response = await handleMessage({
    jsonrpc: "2.0", id: 5, method: "tools/call",
    params: { name: "list_sources", arguments: {} },
  }, api);
  assert.equal(response.result.isError, true);
  assert.match(response.result.content[0].text, /not reachable|start Pane/);
});

test("list_resets derives reset rows and reports unsupported when absent", async () => {
  const api = mockApi({
    listResets: async () => ({ supported: true, count: 1, resets: [{ providerId: "claude", label: "Session", resetsAt: "2026-10-23T12:00:00Z" }] }),
  });
  const response = await handleMessage({
    jsonrpc: "2.0", id: 6, method: "tools/call",
    params: { name: "list_resets", arguments: {} },
  }, api);
  assert.match(response.result.content[0].text, /"supported": true/);
  const unsupported = await handleMessage({
    jsonrpc: "2.0", id: 7, method: "tools/call",
    params: { name: "list_resets", arguments: {} },
  }, mockApi({ listResets: async () => ({ supported: false, resets: [] }) }));
  assert.match(unsupported.result.content[0].text, /"supported": false/);
});
