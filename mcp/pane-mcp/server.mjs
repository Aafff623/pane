#!/usr/bin/env node
// Pane MCP stdio server. A newline-delimited JSON-RPC process; it never
// writes credentials and never asks Pane to refresh.
import { createInterface } from "node:readline";
import { createPaneApi, handleMessage, validateBaseUrl } from "./lib.mjs";

let api;
try {
  const baseUrl = validateBaseUrl(process.env.PANE_API_BASE);
  api = createPaneApi({ baseUrl });
} catch (err) {
  process.stderr.write(`[pane-mcp] ${err.message}\n`);
  process.exit(2);
}

const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) {
  const text = line.trim();
  if (!text) continue;
  let message;
  try {
    message = JSON.parse(text);
  } catch {
    process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "Parse error" } }) + "\n");
    continue;
  }
  try {
    const response = await handleMessage(message, api);
    if (response) process.stdout.write(JSON.stringify(response) + "\n");
  } catch (err) {
    process.stderr.write(`[pane-mcp] ${err?.message || err}\n`);
    if (message?.id !== undefined && message?.id !== null) {
      process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id: message.id, error: { code: -32603, message: "Internal error" } }) + "\n");
    }
  }
}
