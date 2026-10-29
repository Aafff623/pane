export const DESIGN_MODE_SKILL =
  "/.well-known/agent-skills/design-mode/SKILL.md";

export const MCP_SETUP_PROMPT = `Help me connect Design Mode to this AI app. Read https://designmode.app/.well-known/agent-skills/design-mode/SKILL.md and https://designmode.app/docs/mcp-setup first. Check this app's real MCP support and current configuration before changing anything. Prefer Cloud at https://mcp.designmode.app/mcp; explain Local or Self-hosted if needed. Guide me to create an anonymous credential in the extension's MCP page and enter the bearer token locally through a protected client input or environment variable—never ask me to paste it into chat. With my approval, add the correct configuration without overwriting other servers, and tell me if a restart is needed. Discover the tools and test with get_session_summary({}), then get_changes({}) only if I approve reading the page. Do not modify, resolve or clear anything during setup. If this app cannot use the required MCP transport and authentication, explain the limitation and show me how to use Copy as Prompt instead.`;

export const mcpClients = [
  {
    name: "Claude Code",
    location: "Project .mcp.json",
    description:
      "Merge this server into .mcp.json. Set DESIGN_MODE_TOKEN locally in the environment that launches Claude Code, using protected input rather than a literal token in a shell command. Claude Code expands ${DESIGN_MODE_TOKEN} in headers. Review project-server approval, then check /mcp; restart if the new server has not loaded.",
    source: "https://code.claude.com/docs/en/mcp",
    code: JSON.stringify(
      {
        mcpServers: {
          "design-mode": {
            type: "http",
            url: "https://mcp.designmode.app/mcp",
            headers: { Authorization: "Bearer ${DESIGN_MODE_TOKEN}" },
          },
        },
      },
      null,
      2,
    ),
  },
  {
    name: "Cursor",
    location: "Project .cursor/mcp.json or user ~/.cursor/mcp.json",
    description:
      "Merge this entry into your existing servers. Supply DESIGN_MODE_TOKEN through the environment used to launch Cursor; its ${env:NAME} syntax differs from Claude Code. Enable the server in Cursor's MCP settings and review each tool's arguments before approval.",
    source: "https://cursor.com/docs/context/mcp",
    code: JSON.stringify(
      {
        mcpServers: {
          "design-mode": {
            url: "https://mcp.designmode.app/mcp",
            headers: { Authorization: "Bearer ${env:DESIGN_MODE_TOKEN}" },
          },
        },
      },
      null,
      2,
    ),
  },
  {
    name: "VS Code / Copilot",
    location: "Workspace .vscode/mcp.json — local extension-host chat",
    description:
      "Merge both servers and inputs into your configuration. VS Code prompts for the token in a masked input; do not paste it into chat. Start the server with MCP: List Servers. Interactive inputs are not forwarded to Agent Host sessions; for those, follow Microsoft's current configuration reference instead.",
    source:
      "https://code.visualstudio.com/docs/agents/reference/mcp-configuration",
    code: JSON.stringify(
      {
        servers: {
          "design-mode": {
            type: "http",
            url: "https://mcp.designmode.app/mcp",
            headers: { Authorization: "Bearer ${input:designModeToken}" },
          },
        },
        inputs: [
          {
            type: "promptString",
            id: "designModeToken",
            description: "Design Mode bearer token",
            password: true,
          },
        ],
      },
      null,
      2,
    ),
  },
];
