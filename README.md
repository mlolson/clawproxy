# ClawProxy

> **Warning:** This is a learning project, use at your own risk. I still haven't successfully configured OpenClaw to work with it. If you are hoping for it to work out of the box you might be disappointed.

A lightweight HTTP proxy that injects authentication credentials into outbound API requests. Designed to run alongside [OpenClaw](https://github.com/openclaw) to provide AI agents in sandboxed environments (like Docker containers) with authenticated API access without exposing secrets directly.

## How It Works

```
┌─────────────────────────────────────┐
│  Agent Container                    │
│                                     │
│  POST /v1/chat/completions          │
│  X-Upstream-Host: api.openai.com    │
│  Authorization: Bearer PROXY:openai │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│  ClawProxy (host machine)           │
│                                     │
│  1. Read X-Upstream-Host header     │
│  2. Validate against allowlist      │
│  3. Substitute PROXY:openai → sk-xx │
│  4. Forward to api.openai.com       │
└──────────────────┬──────────────────┘
                   │
                   ▼
            ┌─────────────┐
            │ api.openai  │
            │    .com     │
            └─────────────┘
```

**Key benefits:**
- Agents use placeholder tokens (`PROXY:openai`) instead of real API keys
- Secrets stay on the host machine, never exposed to containers
- Upstream hosts are allowlisted - agents can't proxy to arbitrary destinations
- Works with any HTTP client or SDK

## Quick Start

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Build

```bash
cd clawproxy
cargo build --release
```

### 3. Initialize Configuration

```bash
./target/release/clawproxy init
```

This creates:
- `~/.config/clawproxy/config.yaml` - proxy configuration
- `~/.config/clawproxy/secrets/` - directory for API keys

### 4. Add Your API Keys

```bash
# OpenAI
echo 'sk-your-openai-key' | ./target/release/clawproxy secret set openai

# Anthropic
echo 'sk-ant-your-anthropic-key' | ./target/release/clawproxy secret set anthropic

# Or interactively
./target/release/clawproxy secret set openai
```

### 5. Start the Proxy

```bash
./target/release/clawproxy start
```

The proxy listens on `127.0.0.1:8080` by default.

## Usage

### Python with OpenAI SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080",
    api_key="PROXY:openai",
    default_headers={"X-Upstream-Host": "api.openai.com"}
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

### Python with Anthropic SDK

```python
import anthropic

client = anthropic.Anthropic(
    base_url="http://localhost:8080",
    api_key="PROXY:anthropic",
    default_headers={"X-Upstream-Host": "api.anthropic.com"}
)

message = client.messages.create(
    model="claude-sonnet-4-20250514",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello!"}]
)
```

### cURL

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "X-Upstream-Host: api.openai.com" \
  -H "Authorization: Bearer PROXY:openai" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello!"}]}'
```

### Docker Container

When running agents in Docker, use `host.docker.internal` to reach the proxy on the host:

```python
client = OpenAI(
    base_url="http://host.docker.internal:8080",
    api_key="PROXY:openai",
    default_headers={"X-Upstream-Host": "api.openai.com"}
)
```

## CLI Reference

### `clawproxy start`

Start the proxy server.

```bash
clawproxy start [OPTIONS]

Options:
  -c, --config <PATH>  Path to config file (default: ~/.config/clawproxy/config.yaml)
  -p, --port <PORT>    Override listen port
```

### `clawproxy init`

Initialize the configuration directory with example config.

```bash
clawproxy init
```

### `clawproxy secret set <NAME>`

Set a secret. Reads the value from stdin.

```bash
# From pipe
echo 'sk-xxx' | clawproxy secret set openai

# Interactive
clawproxy secret set openai
```

### `clawproxy secret list`

List configured secrets (values are masked).

```bash
clawproxy secret list
```

### `clawproxy secret delete <NAME>`

Delete a secret.

```bash
clawproxy secret delete openai
clawproxy secret delete openai --force  # Skip confirmation
```

## Configuration

Configuration file: `~/.config/clawproxy/config.yaml`

```yaml
listen:
  host: "127.0.0.1"
  port: 8080

# Header that specifies the upstream host
upstream_header: "X-Upstream-Host"

# Service definitions (keyed by upstream host)
# Only hosts listed here are allowed - acts as an allowlist
services:
  api.openai.com:
    secret: "openai"              # Name of secret file
    auth_header: "Authorization"  # Header to inject
    auth_format: "Bearer {secret}" # Format ({secret} is replaced)

  api.anthropic.com:
    secret: "anthropic"
    auth_header: "x-api-key"
    auth_format: "{secret}"

# Token substitution
# If enabled, PROXY:xxx in any header is replaced with the secret value
substitute_tokens: true
token_pattern: "PROXY:([a-zA-Z0-9_-]+)"
```

### Secrets

Secrets are stored as individual files in `~/.config/clawproxy/secrets/`:

```
~/.config/clawproxy/secrets/
├── openai      # Contains: sk-xxxxxxxx
├── anthropic   # Contains: sk-ant-xxxxxxxx
└── github      # Contains: ghp_xxxxxxxx
```

File permissions are set to 600 (owner read/write only).

## Running Tests

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
cargo test --test integration
```

### With Logging

```bash
RUST_LOG=debug cargo test
```

### Test Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin
```

## Docker Integration

### Build Agent Container

```bash
cd docker
docker build -f Dockerfile.agent -t agent .
```

### Run with Docker Compose

```bash
# Start proxy on host first
clawproxy start

# Run agent container
cd docker
docker-compose up
```

### Example docker-compose.yaml

```yaml
version: "3.8"

services:
  agent:
    build:
      context: .
      dockerfile: Dockerfile.agent
    extra_hosts:
      - "host.docker.internal:host-gateway"
    volumes:
      - ./workspace:/workspace
    command: ["python", "agent.py"]
```

## Security

| Concern | Mitigation |
|---------|------------|
| Secrets on disk | File permissions 600, directory 700 |
| Secrets in logs | Auth header values are never logged |
| Arbitrary upstream | Services config acts as allowlist |
| Network sniffing | Proxy → upstream uses TLS |

## Response Codes

| Code | Meaning |
|------|---------|
| (upstream) | Successful proxy, returns upstream status |
| 400 | Missing `X-Upstream-Host` header |
| 403 | Upstream host not in allowlist |
| 502 | Upstream error |
| 503 | Upstream connection refused |
| 504 | Upstream timeout |

## Troubleshooting

### Proxy won't start

Check if the port is already in use:
```bash
lsof -i :8080
```

### Agent can't connect

From Docker, ensure `host.docker.internal` resolves:
```bash
docker run --rm alpine ping host.docker.internal
```

### Secret not found

Verify the secret exists:
```bash
clawproxy secret list
```

### Enable debug logging

```bash
RUST_LOG=debug clawproxy start
```

## License

MIT
