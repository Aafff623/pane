# pane-mcp

Read-only MCP stdio adapter for Pane's existing local API. It does not
refresh providers, read credentials, or expose account PII.

## Run

```sh
node mcp/pane-mcp/server.mjs
```

Requires Node 20+ and a running Pane instance on the default loopback API
at `http://127.0.0.1:6736`. `PANE_API_BASE` may override the loopback origin
for tests; non-loopback hosts are rejected.

## MCP client configuration

Use an absolute path to `server.mjs` in your MCP client:

```json
{
  "mcpServers": {
    "pane": {
      "command": "node",
      "args": ["D:\\code\\pane\\mcp\\pane-mcp\\server.mjs"]
    }
  }
}
```

Tools: `list_providers`, `get_usage`, `get_provider_quota`, `list_sources`,
`get_status`, `list_resets`. Every call is a GET against Pane's read-only
API; results carry `asOf`, `stale` and `ageSecs` when the source provides a
fetch timestamp. `list_resets` returns `unsupported` when Pane has no
structured reset timestamps.

## Privacy

The shim talks only to loopback. It returns sanitized provider metadata and
never returns API keys, tokens, cookies, email addresses, masked keys or raw
config. It has no refresh/test/write tools.
