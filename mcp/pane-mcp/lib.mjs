// Pane MCP server (Phase 1) — a thin, read-only adapter over the local
// 127.0.0.1:6736 usage API. Query shaping lives here; Pane keeps owning
// credentials, refresh and provider fetching. Every call is a GET.

export const TOOLS = [
  {
    name: "list_providers",
    description: "List Pane's enabled providers and their latest usage snapshot summary.",
    inputSchema: { type: "object", properties: {}, additionalProperties: false },
  },
  {
    name: "get_usage",
    description: "Read one provider's latest usage lines, including stale/asOf metadata.",
    inputSchema: {
      type: "object",
      properties: { provider_id: { type: "string", minLength: 1, maxLength: 128 } },
      required: ["provider_id"],
      additionalProperties: false,
    },
  },
  {
    name: "get_provider_quota",
    description: "Read one provider's quota/balance lines without asking Pane to refresh.",
    inputSchema: {
      type: "object",
      properties: { provider_id: { type: "string", minLength: 1, maxLength: 128 } },
      required: ["provider_id"],
      additionalProperties: false,
    },
  },
  {
    name: "list_sources",
    description: "List local spend sources, their detection state and failure reason.",
    inputSchema: { type: "object", properties: {}, additionalProperties: false },
  },
  {
    name: "get_status",
    description: "Read Pane's runtime status and doctor checks with remediation.",
    inputSchema: { type: "object", properties: {}, additionalProperties: false },
  },
  {
    name: "list_resets",
    description: "List known provider reset timestamps; returns unsupported when Pane exposes none.",
    inputSchema: { type: "object", properties: {}, additionalProperties: false },
  },
];

const MAX_BODY_BYTES = 2 * 1024 * 1024;
const DEFAULT_BASE = "http://127.0.0.1:6736";

export function validateBaseUrl(raw) {
  const value = String(raw ?? DEFAULT_BASE).replace(/\/+$/, "");
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error("PANE_API_BASE is not a valid URL");
  }
  if (url.protocol !== "http:") throw new Error("Pane API base must use HTTP");
  if (!["127.0.0.1", "localhost", "[::1]"].includes(url.hostname)) {
    throw new Error("Pane API base must stay on loopback");
  }
  if (url.pathname !== "/" && url.pathname !== "") throw new Error("Pane API base must not contain a path");
  return url.origin;
}

async function readLimitedBody(response) {
  const length = Number(response.headers?.get?.("content-length") || 0);
  if (length > MAX_BODY_BYTES) throw new Error("Pane API response is too large");
  const body = await response.text();
  if (Buffer.byteLength(body, "utf8") > MAX_BODY_BYTES) throw new Error("Pane API response is too large");
  return body;
}

export async function requestJson(path, { baseUrl = DEFAULT_BASE, fetchImpl = globalThis.fetch, timeoutMs = 5000 } = {}) {
  if (!/^\/v1\/[A-Za-z0-9_/?=&@%.:-]+$/.test(path) || path.includes("..")) {
    throw new Error("invalid Pane API path");
  }
  const base = validateBaseUrl(baseUrl);
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchImpl(`${base}${path}`, {
      method: "GET",
      headers: { Accept: "application/json" },
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`Pane API returned HTTP ${response.status}`);
    const body = await readLimitedBody(response);
    try {
      return JSON.parse(body);
    } catch {
      throw new Error("Pane API returned non-JSON data");
    }
  } catch (err) {
    if (err?.name === "AbortError") throw new Error("Pane API request timed out");
    if (String(err?.message || "").includes("fetch failed")) {
      throw new Error("Pane API is not reachable at 127.0.0.1:6736; start Pane first");
    }
    throw err;
  } finally {
    clearTimeout(timer);
  }
}

function safeText(value) {
  if (typeof value !== "string") return value;
  return value
    .replace(/\bBearer\s+[^\s]+/gi, "Bearer <redacted>")
    .replace(/\bsk-[A-Za-z0-9_-]{4,}/g, "sk-<redacted>")
    .replace(/([?&](?:key|token|cookie|authorization)=)[^&\s]+/gi, "$1<redacted>")
    .replace(/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g, "<redacted>");
}

function sanitizeLine(line) {
  if (!line || typeof line !== "object") return {};
  const out = {};
  for (const key of ["type", "label", "limit", "format", "used", "value", "subtitle", "resetsAt", "periodDurationMs", "color"]) {
    if (key in line) out[key] = typeof line[key] === "string" ? safeText(line[key]) : line[key];
  }
  if (line.usage && typeof line.usage === "object") {
    out.usage = {
      unit: line.usage.unit,
      used: line.usage.used,
      total: line.usage.total ?? null,
      scope: line.usage.scope || "",
    };
  }
  return out;
}

export function sanitizeProvider(provider) {
  const fetchedAt = provider?.fetchedAt ?? null;
  const parsed = fetchedAt ? Date.parse(fetchedAt) : NaN;
  const asOf = Number.isFinite(parsed) ? new Date(parsed).toISOString() : null;
  const ageSecs = Number.isFinite(parsed) ? Math.max(0, Math.round((Date.now() - parsed) / 1000)) : null;
  return {
    providerId: provider?.providerId ?? provider?.id ?? "",
    displayName: provider?.displayName ?? provider?.name ?? "",
    plan: provider?.plan ?? null,
    status: provider?.status ?? null,
    stale: Boolean(provider?.stale),
    error: safeText(provider?.error ?? null),
    warning: safeText(provider?.warning ?? null),
    fetchedAt,
    asOf,
    ageSecs,
    lines: (provider?.lines ?? []).map(sanitizeLine),
  };
}

function providerSummary(provider) {
  const row = sanitizeProvider(provider);
  return {
    providerId: row.providerId,
    displayName: row.displayName,
    plan: row.plan,
    status: row.status,
    stale: row.stale,
    asOf: row.asOf,
    ageSecs: row.ageSecs,
    error: row.error,
    warning: row.warning,
  };
}

function requireProviderId(args) {
  const id = typeof args?.provider_id === "string" ? args.provider_id.trim() : "";
  if (!id || id.length > 128 || !/^[A-Za-z0-9@._:-]+$/.test(id)) throw new Error("provider_id is invalid");
  return id;
}

export function createPaneApi({ baseUrl = DEFAULT_BASE, fetchImpl = globalThis.fetch, now = () => Date.now() } = {}) {
  const request = (path) => requestJson(path, { baseUrl, fetchImpl });
  return {
    async listProviders() {
      const payload = await request("/v1/providers");
      const providers = Array.isArray(payload?.providers) ? payload.providers : [];
      return {
        asOf: new Date(now()).toISOString(),
        providers: providers.map(providerSummary),
        disabled: Array.isArray(payload?.disabled) ? payload.disabled.map(String) : [],
      };
    },
    async getUsage(providerId) {
      const payload = await request("/v1/providers");
      const providers = Array.isArray(payload?.providers) ? payload.providers : [];
      let row = providers.find((p) => (p.providerId ?? p.id) === providerId);
      if (!row) {
        const usage = await request(`/v1/usage/${encodeURIComponent(providerId)}`);
        row = usage?.providerId ? usage : { ...usage, providerId };
      }
      return sanitizeProvider(row);
    },
    async getStatus() {
      const [status, doctor] = await Promise.all([request("/v1/status"), request("/v1/doctor")]);
      return { asOf: status?.generatedAt ?? new Date(now()).toISOString(), status, doctor };
    },
    async listSources() {
      return request("/v1/sources");
    },
    async listResets() {
      const payload = await request("/v1/providers");
      const providers = Array.isArray(payload?.providers) ? payload.providers : [];
      const resets = [];
      for (const provider of providers) {
        for (const line of provider.lines ?? []) {
          if (!line?.resetsAt) continue;
          resets.push({
            providerId: provider.providerId ?? provider.id,
            displayName: provider.displayName ?? provider.name,
            label: line.label,
            resetsAt: line.resetsAt,
            periodDurationMs: line.periodDurationMs ?? null,
          });
        }
      }
      if (!resets.length) {
        return { supported: false, reason: "Pane exposes no structured reset timestamps for current providers", resets: [] };
      }
      resets.sort((a, b) => Date.parse(a.resetsAt) - Date.parse(b.resetsAt));
      return { supported: true, count: resets.length, resets };
    },
  };
}

async function callTool(name, args, api) {
  switch (name) {
    case "list_providers":
      return api.listProviders();
    case "get_usage":
      return api.getUsage(requireProviderId(args));
    case "get_provider_quota": {
      const usage = await api.getUsage(requireProviderId(args));
      return { ...usage, lines: usage.lines.filter((line) => line.type === "progress" || line.type === "text") };
    }
    case "list_sources":
      return api.listSources();
    case "get_status":
      return api.getStatus();
    case "list_resets":
      return api.listResets();
    default:
      throw new Error(`unknown tool: ${name}`);
  }
}

export async function handleMessage(message, api = createPaneApi()) {
  if (!message || message.jsonrpc !== "2.0" || typeof message.method !== "string") {
    return { jsonrpc: "2.0", id: message?.id ?? null, error: { code: -32600, message: "Invalid Request" } };
  }
  if (message.id === undefined || message.id === null) return null; // notification
  const { id, method, params } = message;
  try {
    if (method === "initialize") {
      return {
        jsonrpc: "2.0",
        id,
        result: {
          protocolVersion: "2025-06-18",
          capabilities: { tools: {} },
          serverInfo: { name: "pane-mcp", version: "0.1.0" },
        },
      };
    }
    if (method === "ping") return { jsonrpc: "2.0", id, result: {} };
    if (method === "tools/list") return { jsonrpc: "2.0", id, result: { tools: TOOLS } };
    if (method === "tools/call") {
      const name = params?.name;
      try {
        const result = await callTool(name, params?.arguments ?? {}, api);
        return {
          jsonrpc: "2.0",
          id,
          result: { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] },
        };
      } catch (err) {
        return {
          jsonrpc: "2.0",
          id,
          result: {
            content: [{ type: "text", text: safeText(String(err?.message || err)) }],
            isError: true,
          },
        };
      }
    }
    return { jsonrpc: "2.0", id, error: { code: -32601, message: `Method not found: ${method}` } };
  } catch (err) {
    return { jsonrpc: "2.0", id, error: { code: -32603, message: safeText(String(err?.message || err)) } };
  }
}
