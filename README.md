# tg-mcp-server

Telegram MTProto MCP server built on top of `grammers-client` and `rmcp`.

The server provides MCP tools for interacting with Telegram users, groups, channels, messages, participants, comments, and other Telegram entities using the official Telegram MTProto API.

## Requirements

Before running the server, create a Telegram application and obtain:

- `tg_api_id`
- `tg_api_hash`

You can create a Telegram application at:

https://my.telegram.org

---

## Configuration

Create a configuration file based on the following example.

### config.json

```json
{
  "tg_api_id": 00000000,
  "tg_api_hash": "my-telegram-api-hash",
  "phone_number": "+12233220078",
  "session_file": null
}
```

### Parameters

| Field | Description |
|---------|-------------|
| `tg_api_id` | Telegram application API ID |
| `tg_api_hash` | Telegram application API hash |
| `phone_number` | Phone number used for Telegram authorization |
| `session_file` | Optional path to a Telegram session file. If `null`, the default session file location will be used. |

### Session File

The `session_file` parameter allows you to customize where Telegram session data is stored.

Example:

```json
{
  "tg_api_id": 12345678,
  "tg_api_hash": "xxxxxxxxxxxxxxxxxxxxxxxx",
  "phone_number": "+12233220078",
  "session_file": "./sessions/my_account.session"
}
```

If `session_file` is set to `null`, the server will use its default session file location.

---

## Usage

### Command Line

```text
Usage: tg-mcp-server [OPTIONS] --config-path <CONFIG_PATH> --transport <TRANSPORT>

Options:
      --config-path <CONFIG_PATH>
      --transport <TRANSPORT>
      --mcp-host <MCP_HOST>        [default: 127.0.0.1]
      --mcp-port <MCP_PORT>        [default: 9050]
  -h, --help                       Print help
  -V, --version                    Print version
```

### Arguments

| Option | Description |
|----------|-------------|
| `--config-path` | Path to configuration file |
| `--transport` | MCP transport (`stdio` or `streamable-http`) |
| `--mcp-host` | Host address used by HTTP transport |
| `--mcp-port` | Port used by HTTP transport |

---

## Examples

### STDIO Transport

```bash
tg-mcp-server \
    --config-path ./config.json \
    --transport stdio
```

### Streamable HTTP Transport

```bash
tg-mcp-server \
    --config-path ./config.json \
    --transport streamable-http \
    --mcp-host 127.0.0.1 \
    --mcp-port 9050
```

---

## First Run

During the first launch the server will:

1. Connect to Telegram.
2. Send a login code to the configured phone number.
3. Request the verification code.
4. Create a Telegram session file.

Subsequent launches will reuse the existing session and will not require re-authentication unless the session is revoked or deleted.

---

## Security Notice

⚠️ **Important**

The following files contain sensitive information and must never be:

- committed to Git repositories;
- shared publicly;
- attached to GitHub issues;
- included in bug reports;
- exposed through logs;
- published in Docker images or backups.

### config.json

Contains:

- Telegram API ID;
- Telegram API hash;
- phone number;
- optional session file location.

### Telegram Session File

By default:

```text
tg_mcp_server.session
```

or a custom path specified in:

```json
{
  "session_file": "./path/to/session.session"
}
```

The session file contains Telegram authentication credentials.

Anyone with access to the session file may be able to authenticate as your Telegram account without requiring a login code.

### Recommended .gitignore

```gitignore
config.json
*.session
sessions/
```

⚠️ Never publish session files, API hashes, authorization codes, or Telegram credentials.

---

## Disclaimer

This project uses the Telegram MTProto API and operates on behalf of the authenticated Telegram account.

You are responsible for protecting your Telegram credentials and complying with Telegram Terms of Service
