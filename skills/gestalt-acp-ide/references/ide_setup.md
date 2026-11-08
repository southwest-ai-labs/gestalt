# IDE Setup Reference

This document provides detailed syntax references and protocol verification commands for Gestalt ACP integration.

## IDE Configuration Syntax

### Standard Options

The typical stdio configuration syntax across most IDEs follows this structure:
```json
{
  "command": "<executable_name_or_path>",
  "args": ["<arg1>", "<arg2>"],
  "type": "stdio"
}
```

### SSE Options

For SSE configurations, the syntax usually requires a URL:
```json
{
  "url": "<server_url>",
  "type": "sse"
}
```

## Protocol Verification Commands

To verify that Gestalt is correctly emitting the ACP protocol, you can use standard command-line tools.

### Testing Stdio

Run the following command in your terminal to see if Gestalt responds correctly via stdio:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' | gestalt acp serve --stdio
```
You should see a valid JSON-RPC initialization response from Gestalt.

### Testing SSE

If you have an SSE server running (e.g., on port 8080), you can test it with `curl`:
```bash
curl -N http://localhost:8080/sse
```
You should see an event stream connecting and keeping the connection open.
