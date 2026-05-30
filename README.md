# tg-mcp-server

Telegram MTProto MCP server built on top of `grammers-client` and `rmcp`.

The server provides MCP tools for interacting with Telegram users, groups, channels, messages, participants, comments, and other Telegram entities using the official Telegram API (MTProto).

## Requirements

Before running the server, create a Telegram application and obtain:

- `tg_api_id`
- `tg_api_hash`

You can create an application at:

https://my.telegram.org

---

## Configuration

Create a configuration file based on the following example:

### config.json

```json
{
  "tg_api_id": 00000000,
  "tg_api_hash": "my-telegram-api-hash",
  "phone_number": "+12233220078"
}
```

### Parameters

| Field | Description |
|---------|-------------|
| `tg_api_id` | Telegram application API ID |
| `tg_api_hash` | Telegram application API hash |
| `phone_number` | Phone number used for Telegram authorization |

---

## Security Notice

⚠️ **Important**

The following files contain sensitive information and must never be committed to source control or shared publicly:

### config.json

Contains:

- Telegram API ID
- Telegram API hash
- Phone number

### tg_mcp_server.session

Contains Telegram session credentials used to authenticate your account.

Anyone with access to these files may be able to access your Telegram account.

Recommended `.gitignore` entries:

```gitignore
config.json
*.session
tg_mcp_server.session
```

---

## Usage

```bash
tg-mcp-server \
    --config-path ./config.json \
    --transport stdio
```

### Command Line Options

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

### STDIO transport

```bash
tg-mcp-server \
    --config-path ./config.json \
    --transport stdio
```

### Streamable HTTP transport

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
4. Create a local session file (`tg_mcp_server.session`).

Subsequent launches will reuse the existing session and will not require re-authentication unless the session is revoked.

---

## Disclaimer

This project uses the Telegram MTProto API and operates on behalf of the authenticated Telegram account. Ensure that you comply with Telegram Terms of Service and protect all authentication credentials.
