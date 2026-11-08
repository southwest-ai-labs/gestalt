---
name: gestalt-acp-ide
description: Register Gestalt as an ACP coding agent in various IDEs.
---

# Gestalt ACP IDE Skill

This skill provides configurations to register Gestalt as an ACP (Agent Control Protocol) coding agent in popular IDEs.

## Configurations

### Claude Code

To add Gestalt as an agent in Claude Code, run the following command in your terminal:

```bash
claude config add-agent gestalt "gestalt acp serve --stdio"
```

### Cursor

Add the following ACP subagent configuration to your `.cursor/settings.json` or `.cursor/mcp.json` file:

```json
{
  "mcp": {
    "gestalt": {
      "command": "gestalt",
      "args": ["acp", "serve", "--stdio"],
      "type": "stdio"
    }
  }
}
```

### Zed

Add the following agent block to your Zed `settings.json` file:

```json
{
  "agents": {
    "gestalt": {
      "command": "gestalt",
      "args": ["acp", "serve", "--stdio"]
    }
  }
}
```

### VS Code

For VS Code (and Windsurf), you can configure Gestalt via JSON-RPC stdio or remote SSE connection.

#### JSON-RPC stdio

Add the following to your VS Code `settings.json`:

```json
{
  "mcp.agents": {
    "gestalt": {
      "command": "gestalt",
      "args": ["acp", "serve", "--stdio"]
    }
  }
}
```

#### Remote SSE Connection

If you are running Gestalt on a remote server, use the SSE connection configuration:

```json
{
  "mcp.agents": {
    "gestalt": {
      "url": "http://localhost:8080/sse",
      "type": "sse"
    }
  }
}
```

## Troubleshooting

### Broken Stdio Pipes

If your IDE reports that the stdio pipe is broken:
1. Ensure the `gestalt` executable is in your system's `PATH`.
2. Try providing the absolute path to the `gestalt` binary in the configuration instead of just `gestalt`.
3. Check for any conflicting extensions or background processes blocking stdio streams.
4. Verify that you don't have multiple instances of Gestalt competing for the same standard streams.

### Connection Timeouts

If you encounter connection timeouts (especially with SSE):
1. Ensure that the Gestalt server is actively running (`gestalt acp serve --port 8080`).
2. Verify that the port is not blocked by a firewall.
3. Check if your IDE requires proxy settings to access localhost.
4. Restart the IDE and the Gestalt server to clear any zombie processes.
