# ClawProxy - Rust Implementation Plan v2

## Overview

A secure credential injection system for AI agents, consisting of two components:

1. **clawproxy** — HTTP proxy that injects API credentials into requests
2. **clawproxy-run** — Sandboxed launcher that runs agents without access to secrets

Supports macOS and Linux out of the box, no Docker required.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│   $ clawproxy-run ./openclaw --task "do something"                          │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  LAUNCHER (clawproxy-run)                                             │  │
│  │                                                                       │  │
│  │  1. Detect OS                                                         │  │
│  │  2. Apply sandbox:                                                    │  │
│  │     ┌─────────────────────┬─────────────────────┐                    │  │
│  │     │ macOS               │ Linux               │                    │  │
│  │     ├─────────────────────┼─────────────────────┤                    │  │
│  │     │ sandbox-exec with   │ Landlock rules      │                    │  │
│  │     │ deny file-read* on  │ blocking access to  │                    │  │
│  │     │ secrets directory   │ secrets directory   │                    │  │
│  │     └─────────────────────┴─────────────────────┘                    │  │
│  │  3. Set environment: HTTP_PROXY, HTTPS_PROXY                          │  │
│  │  4. exec() into agent process                                         │  │
│  │                                                                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                 │                                           │
│                                 ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  AGENT PROCESS (sandboxed)                                            │  │
│  │                                                                       │  │
│  │  - Inherits sandbox restrictions                                      │  │
│  │  - Cannot read ~/.config/clawproxy/secrets/*                         │  │
│  │  - Environment has HTTP_PROXY=http://127.0.0.1:8080                  │  │
│  │  - Makes requests with "Authorization: Bearer PROXY:openai"          │  │
│  │                                                                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                 │                                           │
│                                 ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  PROXY (clawproxy, separate process)                                  │  │
│  │                                                                       │  │
│  │  - Listens on 127.0.0.1:8080                                         │  │
│  │  - Receives: POST /openai/v1/chat/completions                        │  │
│  │              Authorization: Bearer PROXY:openai                       │  │
│  │                                                                       │  │
│  │  - Substitutes: PROXY:openai → sk-xxxxxxxx                           │  │
│  │  - Forwards to: https://api.openai.com/v1/chat/completions           │  │
│  │                                                                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                 │                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  SECRETS (filesystem, readable only by proxy)                         │  │
│  │                                                                       │  │
│  │  ~/.config/clawproxy/secrets/                                        │  │
│  │  ├── openai                                                          │  │
│  │  ├── anthropic                                                       │  │
│  │  └── github                                                          │  │
│  │                                                                       │  │
│  │  Agent process CANNOT read these (sandbox enforced)                  │  │
│  │  Proxy process CAN read these (not sandboxed)                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Usage

```bash
# One-time setup
clawproxy init
clawproxy secret set openai    # Prompts for API key
clawproxy secret set anthropic

# Terminal 1: Start proxy
clawproxy start

# Terminal 2: Run agent (sandboxed)
clawproxy-run ./openclaw --task "build me a website"

# Or run any command sandboxed
clawproxy-run python my_agent.py
clawproxy-run npm run agent
```

---

## Project Structure

```
clawproxy/
├── Cargo.toml
├── README.md
├── src/
│   ├── bin/
│   │   ├── clawproxy.rs         # Proxy + CLI entrypoint
│   │   └── clawproxy_run.rs     # Launcher entrypoint
│   ├── lib.rs                   # Shared library code
│   ├── config.rs                # Config + secrets loading
│   ├── proxy/
│   │   ├── mod.rs
│   │   ├── server.rs            # HTTP server
│   │   ├── router.rs            # Route matching
│   │   └── substitution.rs      # Token replacement
│   ├── sandbox/
│   │   ├── mod.rs               # OS detection + dispatch
│   │   ├── macos.rs             # sandbox-exec implementation
│   │   └── linux.rs             # Landlock implementation
│   └── error.rs
├── sandbox-profiles/
│   └── deny-secrets.sb          # macOS sandbox profile template
├── tests/
│   ├── proxy_tests.rs
│   ├── sandbox_tests.rs
│   └── integration/
└── docker/                       # Optional, for CI or users who prefer Docker
    ├── Dockerfile.agent
    └── docker-compose.yaml
```

---

## Dependencies

```toml
[package]
name = "clawproxy"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "clawproxy"
path = "src/bin/clawproxy.rs"

[[bin]]
name = "clawproxy-run"
path = "src/bin/clawproxy_run.rs"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP
axum = "0.7"
reqwest = { version = "0.12", features = ["rustls-tls", "stream"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Utilities
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
regex = "1"
dirs = "5"

# Sandboxing (Linux)
[target.'cfg(target_os = "linux")'.dependencies]
landlock = "0.3"

# Process execution
nix = { version = "0.28", features = ["process", "signal"] }

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## Component Specifications

### 1. Sandbox Module

#### sandbox/mod.rs — OS Dispatch

```rust
// Detect OS at runtime (or compile time) and apply appropriate sandbox

pub struct SandboxConfig {
    /// Paths the sandboxed process cannot read
    pub deny_read: Vec<PathBuf>,
    /// Paths the sandboxed process cannot write
    pub deny_write: Vec<PathBuf>,
    /// Environment variables to set
    pub env: HashMap<String, String>,
}

pub trait Sandbox {
    /// Apply sandbox restrictions and exec into the target command.
    /// This function does not return on success (replaces current process).
    fn exec_sandboxed(config: &SandboxConfig, cmd: &str, args: &[&str]) -> Result<!, SandboxError>;
}

/// Auto-detect OS and return appropriate sandbox implementation
pub fn create_sandbox() -> Box<dyn Sandbox> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOsSandbox);
    
    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxSandbox);
}
```

#### sandbox/macos.rs — sandbox-exec Implementation

```rust
// macOS implementation using sandbox-exec

pub struct MacOsSandbox;

impl Sandbox for MacOsSandbox {
    fn exec_sandboxed(config: &SandboxConfig, cmd: &str, args: &[&str]) -> Result<!, SandboxError> {
        // 1. Generate sandbox profile from template
        let profile = generate_profile(config)?;
        
        // 2. Write profile to temp file
        let profile_path = write_temp_profile(&profile)?;
        
        // 3. Build command: sandbox-exec -f <profile> <cmd> <args...>
        // 4. Set environment variables
        // 5. exec() into sandbox-exec
    }
}

fn generate_profile(config: &SandboxConfig) -> String {
    // Base profile that allows most operations
    let mut profile = String::from(r#"
(version 1)
(allow default)
"#);
    
    // Add deny rules for each path
    for path in &config.deny_read {
        profile.push_str(&format!(r#"
(deny file-read* (subpath "{}"))
"#, path.display()));
    }
    
    for path in &config.deny_write {
        profile.push_str(&format!(r#"
(deny file-write* (subpath "{}"))
"#, path.display()));
    }
    
    profile
}
```

#### sandbox/linux.rs — Landlock Implementation

```rust
// Linux implementation using Landlock

use landlock::{
    Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreated, RulesetStatus, ABI,
};

pub struct LinuxSandbox;

impl Sandbox for LinuxSandbox {
    fn exec_sandboxed(config: &SandboxConfig, cmd: &str, args: &[&str]) -> Result<!, SandboxError> {
        // 1. Check Landlock availability
        let abi = ABI::V3;  // or detect best available
        
        // 2. Create ruleset that allows everything by default
        let mut ruleset = Ruleset::default()
            .handle_access(AccessFs::from_all(abi))?
            .create()?;
        
        // 3. Add rules for allowed paths (everything except denied)
        //    Landlock is allowlist-based, so we need to allow root
        //    then the denied paths simply aren't in the ruleset
        
        // 4. Restrict self
        ruleset.restrict_self()?;
        
        // 5. Set environment variables
        for (k, v) in &config.env {
            std::env::set_var(k, v);
        }
        
        // 6. exec() into target command
        let err = exec::execvp(cmd, args);
        Err(SandboxError::ExecFailed(err))
    }
}
```

**Note on Landlock**: Landlock is allowlist-based (you specify what IS allowed), while macOS sandbox-exec is denylist-based (you specify what is NOT allowed). The Linux implementation needs to:

1. Allow access to most of the filesystem
2. Explicitly NOT allow the secrets directory

This is a bit tricky but well-documented in Landlock examples.

#### Fallback for older Linux

```rust
impl LinuxSandbox {
    fn exec_sandboxed(...) -> Result<!, SandboxError> {
        // Try Landlock first
        if landlock_available() {
            return self.exec_with_landlock(config, cmd, args);
        }
        
        // Fall back to warning + proceed without sandbox
        eprintln!("WARNING: Landlock not available (kernel 5.13+ required)");
        eprintln!("WARNING: Secrets directory is NOT protected");
        eprintln!("Consider using Docker or upgrading your kernel.");
        
        // Still set env and exec, just without protection
        for (k, v) in &config.env {
            std::env::set_var(k, v);
        }
        exec::execvp(cmd, args)
    }
}
```

---

### 2. Launcher Binary (clawproxy-run)

```rust
// src/bin/clawproxy_run.rs

use clap::Parser;
use clawproxy::{config::Config, sandbox};

#[derive(Parser)]
#[command(name = "clawproxy-run")]
#[command(about = "Run a command in a sandbox without access to API secrets")]
struct Cli {
    /// Command to run
    #[arg(required = true)]
    command: String,
    
    /// Arguments to pass to command
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
    
    /// Config file path
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Proxy URL (default: http://127.0.0.1:8080)
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    proxy: String,
    
    /// Skip sandbox (dangerous, for debugging)
    #[arg(long, hide = true)]
    no_sandbox: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Load config to find secrets directory
    let config = Config::load(cli.config.as_deref())?;
    let secrets_dir = config.secrets_dir();
    
    // Build sandbox config
    let sandbox_config = sandbox::SandboxConfig {
        deny_read: vec![secrets_dir.clone()],
        deny_write: vec![secrets_dir.clone()],
        env: HashMap::from([
            ("HTTP_PROXY".into(), cli.proxy.clone()),
            ("HTTPS_PROXY".into(), cli.proxy.clone()),
            ("http_proxy".into(), cli.proxy.clone()),
            ("https_proxy".into(), cli.proxy.clone()),
        ]),
    };
    
    if cli.no_sandbox {
        eprintln!("WARNING: Running without sandbox protection!");
        // Just exec without sandbox
        let err = exec::execvp(&cli.command, &cli.args);
        return Err(err.into());
    }
    
    // Apply sandbox and exec (does not return on success)
    let sandbox = sandbox::create_sandbox();
    let args: Vec<&str> = cli.args.iter().map(|s| s.as_str()).collect();
    sandbox.exec_sandboxed(&sandbox_config, &cli.command, &args)?;
    
    unreachable!()
}
```

---

### 3. Proxy Binary (clawproxy)

Unchanged from previous plan — handles:
- `clawproxy init` — Create config directory
- `clawproxy secret set/list/delete` — Manage secrets
- `clawproxy start` — Run proxy server

---

### 4. Config Updates

```yaml
# ~/.config/clawproxy/config.yaml

listen:
  host: "127.0.0.1"
  port: 8080

# Secrets directory (relative to config dir, or absolute)
secrets_dir: "secrets"

services:
  openai:
    prefix: "/openai"
    upstream: "https://api.openai.com"
    secret: "openai"
    auth_header: "Authorization"
    auth_format: "Bearer {secret}"
  
  anthropic:
    prefix: "/anthropic"
    upstream: "https://api.anthropic.com"
    secret: "anthropic"
    auth_header: "x-api-key"
    auth_format: "{secret}"
  
  github:
    prefix: "/github"
    upstream: "https://api.github.com"
    secret: "github"
    auth_header: "Authorization"
    auth_format: "token {secret}"
```

---

## Execution Tasks

### Phase 1: Project Setup

#### Task 1.1: Initialize Cargo workspace
```
- cargo new clawproxy
- Configure Cargo.toml with two binaries
- Set up directory structure
- Add dependencies
- Verify: cargo check
```

#### Task 1.2: Set up shared library structure
```
- Create src/lib.rs exporting modules
- Create src/error.rs with error types
- Create empty module files
- Verify: cargo check
```

#### Task 1.3: Set up logging
```
- Add tracing initialization
- Configure env filter
- Verify: RUST_LOG=debug cargo run
```

### Phase 2: Configuration (unchanged from v1)

#### Task 2.1: Implement config loading
#### Task 2.2: Implement secrets loading
#### Task 2.3: Implement config validation

### Phase 3: Sandbox Implementation

#### Task 3.1: Define sandbox trait and types
```
- Create src/sandbox/mod.rs
- Define SandboxConfig struct
- Define Sandbox trait
- Define SandboxError enum
- Implement OS detection
```

#### Task 3.2: Implement macOS sandbox
```
- Create src/sandbox/macos.rs
- Implement profile generation from SandboxConfig
- Write profile to temp file
- Build sandbox-exec command
- Implement exec with nix crate
- Test manually: cargo run --bin clawproxy-run -- ls /path/to/secrets
  Should fail with permission denied
```

#### Task 3.3: Implement Linux sandbox
```
- Create src/sandbox/linux.rs
- Add landlock crate dependency (cfg-gated)
- Implement Landlock ruleset creation
- Handle case where secrets dir doesn't need explicit deny
  (Landlock allowlist approach)
- Implement fallback for older kernels
- Test on Linux VM or container
```

#### Task 3.4: Implement launcher binary
```
- Create src/bin/clawproxy_run.rs
- Parse CLI arguments
- Load config for secrets path
- Build SandboxConfig
- Call sandbox.exec_sandboxed()
- Test end-to-end
```

#### Task 3.5: Test sandbox effectiveness
```
- Write test that:
  1. Creates temp secrets directory with test file
  2. Runs clawproxy-run with a command that tries to read the file
  3. Verifies the command fails to read
- Run on both macOS and Linux
```

### Phase 4: Proxy Implementation (unchanged from v1)

#### Task 4.1: Implement router
#### Task 4.2: Implement token substitution
#### Task 4.3: Implement HTTP server
#### Task 4.4: Implement request forwarding
#### Task 4.5: Implement response streaming
#### Task 4.6: Implement error handling

### Phase 5: CLI Implementation

#### Task 5.1: Implement `clawproxy init`
```
- Create ~/.config/clawproxy/ directory
- Create secrets/ subdirectory with mode 700
- Write default config.yaml
- Print next steps
```

#### Task 5.2: Implement `clawproxy secret set <n>`
```
- Read from stdin or prompt
- Write to secrets/<n> with mode 600
- Print confirmation
```

#### Task 5.3: Implement `clawproxy secret list`
```
- List secrets directory
- Show masked preview
- Show which services use each
```

#### Task 5.4: Implement `clawproxy secret delete`

#### Task 5.5: Implement `clawproxy start`
```
- Load config and secrets
- Validate
- Start server
- Handle signals for graceful shutdown
```

#### Task 5.6: Implement `clawproxy status`
```
- Check if proxy is running
- Show listen address
- Show loaded services
- Test connectivity
```

### Phase 6: Testing

#### Task 6.1: Unit tests — Config
```
- Valid config parsing
- Missing file handling
- Invalid YAML handling
```

#### Task 6.2: Unit tests — Router
```
- Prefix matching
- URL rewriting
- Query string preservation
```

#### Task 6.3: Unit tests — Substitution
```
- Token replacement
- Multiple tokens
- Unknown token handling
```

#### Task 6.4: Integration tests — Proxy
```
- Use wiremock to mock upstream
- Verify auth injection
- Verify token substitution
- Verify error responses
```

#### Task 6.5: Integration tests — Sandbox
```
- Test macOS sandbox blocks file read
- Test Linux Landlock blocks file read
- Test fallback warning on old kernel
```

#### Task 6.6: End-to-end tests
```
- Start proxy
- Run sandboxed command that makes API request
- Verify request succeeds
- Verify sandboxed command cannot read secrets
```

### Phase 7: Distribution

#### Task 7.1: Build release binaries
```
- cargo build --release
- Test on macOS (Intel + Apple Silicon)
- Test on Linux (x86_64)
```

#### Task 7.2: Create install script
```bash
#!/bin/bash
# install.sh

VERSION="0.1.0"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map architecture
case $ARCH in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
esac

URL="https://github.com/you/clawproxy/releases/download/v${VERSION}/clawproxy-${OS}-${ARCH}.tar.gz"

curl -L "$URL" | tar xz -C /usr/local/bin/
echo "Installed clawproxy and clawproxy-run to /usr/local/bin/"
```

#### Task 7.3: Create Homebrew formula (macOS)
```ruby
class clawproxy < Formula
  desc "HTTP proxy with credential injection for AI agents"
  homepage "https://github.com/you/clawproxy"
  url "https://github.com/you/clawproxy/archive/v0.1.0.tar.gz"
  sha256 "..."
  
  depends_on "rust" => :build
  
  def install
    system "cargo", "build", "--release"
    bin.install "target/release/clawproxy"
    bin.install "target/release/clawproxy-run"
  end
end
```

#### Task 7.4: Document installation
```markdown
## Installation

### macOS (Homebrew)
brew install you/tap/clawproxy

### Linux / macOS (script)
curl -fsSL https://raw.githubusercontent.com/you/clawproxy/main/install.sh | bash

### From source
cargo install --git https://github.com/you/clawproxy
```

### Phase 8: Documentation

#### Task 8.1: Write README.md
```
- Project overview and motivation
- Quick start (3 commands)
- How it works (diagram)
- Configuration reference
- Security model
- Platform support
```

#### Task 8.2: Write SECURITY.md
```
- Threat model
- What is/isn't protected
- Sandbox limitations
- Recommendations
```

#### Task 8.3: Add --help documentation
```
- Comprehensive help for all commands
- Examples in help text
```

---

## Platform Support Matrix

| Feature | macOS 12+ | Linux (kernel 5.13+) | Linux (older) |
|---------|-----------|----------------------|---------------|
| Proxy | ✓ | ✓ | ✓ |
| Sandbox | ✓ (sandbox-exec) | ✓ (Landlock) | ⚠ (warning only) |
| File permissions | ✓ | ✓ | ✓ |

---

## Security Model

### What's protected

| Attack | Protected | Mechanism |
|--------|-----------|-----------|
| Agent reads secrets file | ✓ | Sandbox blocks file access |
| Agent `cat ~/.config/clawproxy/secrets/*` | ✓ | Sandbox blocks |
| Agent spawns subprocess to read secrets | ✓ | Subprocess inherits sandbox |
| Prompt injection tries to exfiltrate keys | ✓ | Keys never in agent's memory |
| Agent logs auth headers | ✓ | Only sees PROXY:xxx tokens |

### What's NOT protected

| Attack | Protected | Why |
|--------|-----------|-----|
| Agent makes expensive API calls | ✗ | Has access via proxy |
| Agent uses API maliciously | ✗ | Proxy just forwards |
| Kernel exploit escapes sandbox | ✗ | Requires kernel hardening |
| Agent reads secrets before sandbox applied | ✗ | Sandbox applied before exec |
| Someone runs agent without clawproxy-run | ✗ | User error |

### Sandbox limitations

**macOS (sandbox-exec)**:
- Technically deprecated by Apple (but still works, used by system)
- Profile syntax is not officially documented
- May break in future macOS versions

**Linux (Landlock)**:
- Requires kernel 5.13+ (Ubuntu 22.04+, Fedora 34+)
- ABI may gain features over time (forward compatible)
- Some filesystems may not support it

---

## Example Session

```bash
# Install
$ brew install you/tap/clawproxy

# Initialize
$ clawproxy init
Created /Users/alice/.config/clawproxy/
Created /Users/alice/.config/clawproxy/secrets/
Created /Users/alice/.config/clawproxy/config.yaml

Add your API keys:
  clawproxy secret set openai
  clawproxy secret set anthropic

Then start the proxy:
  clawproxy start

# Add secrets
$ clawproxy secret set openai
Enter secret for 'openai': ****************************
Saved secret 'openai'

$ clawproxy secret set anthropic
Enter secret for 'anthropic': ****************************
Saved secret 'anthropic'

$ clawproxy secret list
openai      sk-xxxxxxxx...  (used by: openai)
anthropic   sk-ant-xxxx...  (used by: anthropic)

# Start proxy (in background or separate terminal)
$ clawproxy start
INFO  clawproxy > Listening on 127.0.0.1:8080
INFO  clawproxy > Loaded 2 services: openai, anthropic
INFO  clawproxy > Ready

# Run agent (sandboxed)
$ clawproxy-run ./my-agent
INFO  clawproxy-run > Sandbox: macOS (sandbox-exec)
INFO  clawproxy-run > Blocking access to: /Users/alice/.config/clawproxy/secrets
INFO  clawproxy-run > Setting HTTP_PROXY=http://127.0.0.1:8080
INFO  clawproxy-run > Executing: ./my-agent
[agent output...]

# Verify sandbox works
$ clawproxy-run cat ~/.config/clawproxy/secrets/openai
cat: /Users/alice/.config/clawproxy/secrets/openai: Operation not permitted
```

---

## Time Estimate

| Phase | Estimate |
|-------|----------|
| Phase 1: Project Setup | 1-2 hours |
| Phase 2: Configuration | 2-3 hours |
| Phase 3: Sandbox | 4-6 hours |
| Phase 4: Proxy | 4-6 hours |
| Phase 5: CLI | 2-3 hours |
| Phase 6: Testing | 4-6 hours |
| Phase 7: Distribution | 2-3 hours |
| Phase 8: Documentation | 2-3 hours |
| **Total** | **21-32 hours** |

---

## Implementation tips from Claude


Phase 3 (Sandbox) is probably the trickiest part, especially getting Landlock's allowlist model to work like a denylist

A few tips for the implementation:

Start with the proxy (Phase 4) before the sandbox—easier to test and you can use it immediately with manual proxy env vars

Test sandbox-exec manually first before writing Rust code:
bash
   sandbox-exec -p '(version 1)(allow default)(deny file-read* (subpath "/tmp/test"))' cat /tmp/test/secret

Landlock has good examples in the crate docs—the "allow everything except X" pattern is documented



## Open Questions

1. **Daemon mode**: Should `clawproxy start` support `-d` for daemon mode, or rely on systemd/launchd?

2. **Multiple configs**: Support `--profile` flag for different configs (work vs personal)?

3. **Proxy authentication**: Should the proxy itself require auth from the agent (defense in depth)?

4. **Audit logging**: Log all requests to a file for security review?

5. **Graceful sandbox fallback**: On older Linux, should we refuse to run (secure) or warn and continue (usable)?
