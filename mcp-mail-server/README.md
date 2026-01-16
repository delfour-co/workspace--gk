# mcp-mail-server - MCP Server for Email Operations

MCP (Model Context Protocol) server that exposes mail-rs functionality to AI assistants.

## Overview

This server implements the MCP protocol (JSON-RPC 2.0) to allow LLMs to interact with the email system through standardized tool calls.

## Features

- 7 email tools exposed via MCP protocol
- JSON-RPC 2.0 compliant
- Integration with mail-rs maildir storage
- SMTP client for sending emails

## Available Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `send_email` | Send email via SMTP | from, to, subject, body |
| `list_emails` | List emails from mailbox | email, limit (optional) |
| `read_email` | Read email content | email, email_id |
| `search_emails` | Search in emails | email, query |
| `mark_as_read` | Mark email as read | email, email_id |
| `delete_email` | Delete an email | email, email_id |
| `get_email_count` | Count unread emails | email |

## Quick Start

```bash
# Start the server
cargo run

# Or with custom SMTP settings
SMTP_HOST=localhost SMTP_PORT=2525 cargo run
```

Server starts on `http://localhost:8090`

## API Endpoints

### Health Check

```bash
curl http://localhost:8090/health
# {"status":"ok"}
```

### MCP Protocol

```bash
# List available tools
curl -X POST http://localhost:8090/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }'

# Call a tool
curl -X POST http://localhost:8090/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "list_emails",
      "arguments": {
        "email": "user@example.com",
        "limit": 10
      }
    },
    "id": 2
  }'
```

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `SMTP_HOST` | 127.0.0.1 | SMTP server host |
| `SMTP_PORT` | 2525 | SMTP server port |
| `MAILDIR_PATH` | ../mail-rs/data/maildir | Maildir storage path |

## Testing

```bash
# Run tests
cargo test

# Run with verbose output
cargo test -- --nocapture
```

## Architecture

```
┌──────────────┐      ┌─────────────────┐      ┌──────────────┐
│  ai-runtime  │ ──── │ mcp-mail-server │ ──── │   mail-rs    │
│   (LLM)      │ MCP  │   (Port 8090)   │      │ (SMTP/IMAP)  │
└──────────────┘      └─────────────────┘      └──────────────┘
```

## MCP Protocol Reference

### Request Format

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": { ... }
  },
  "id": 1
}
```

### Response Format

```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid request"
  },
  "id": 1
}
```

## Related

- [ai-runtime](../ai-runtime/README.md) - AI orchestration engine
- [mail-rs](../mail-rs/README.md) - SMTP/IMAP server
- [MCP Specification](https://spec.modelcontextprotocol.io/)
