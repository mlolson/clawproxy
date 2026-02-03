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
│   $ clawproxy-run ./openclaw gateway                          │
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

# Configure existing openclaw installation
clawproxy configure-openclaw
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
- Create OS-specific service files:
  - macOS: Write ~/Library/LaunchAgents/ai.clawproxy.plist
    - RunAtLoad: true
    - KeepAlive: true
    - ProgramArguments: [/path/to/clawproxy, start]
    - StandardOutPath/StandardErrorPath for logging
  - Linux: Write ~/.config/systemd/user/clawproxy.service
    - ExecStart=/path/to/clawproxy start
    - Restart=on-failure
    - WantedBy=default.target
- Register/enable the service (but don't start):
  - macOS: launchctl load (with disabled flag or just inform user)
  - Linux: systemctl --user daemon-reload && systemctl --user enable clawproxy
- Print next steps (including how to start the service)
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

#### Task 5.7: Implement `clawproxy configure-openclaw`
```
Prerequisites:
- OpenClaw must already be installed and its daemon service configured
- ClawProxy must be initialized (`clawproxy init` already run)

Steps:

Part A: Update OpenClaw Configuration
1. Locate OpenClaw config file (~/.openclaw/openclaw.json)
2. Verify it exists (error if not: "OpenClaw config not found. Run 'openclaw onboard' first.")
3. Back up original (~/.openclaw/openclaw.json.pre-clawproxy)
4. Parse the JSON and for each provider in models.providers:
   - If apiKey contains a real key (not already PROXY:xxx):
     - Prompt: "Found [provider] API key. Migrate to ClawProxy? [Y/n]"
     - If yes: Save to clawproxy secrets (clawproxy secret set [provider])
   - Update baseUrl to http://127.0.0.1:8080/[provider]
   - Update apiKey to PROXY:[provider]
5. Write modified config
6. Check for credentials in other locations:
   - ~/.openclaw/credentials/oauth.json
   - Warn user if real keys found: "Found keys in oauth.json - remove manually after verifying integration works"

Part B: Modify Daemon Service
7. Detect OS (macOS vs Linux)
8. Locate OpenClaw's service file:
   - macOS: ~/Library/LaunchAgents/ai.openclaw.gateway.plist
   - Linux: ~/.config/systemd/user/openclaw-gateway.service
9. Verify the file exists (error if not: "OpenClaw daemon not found.")
10. Back up the original file (.pre-clawproxy)
11. Parse and modify:
    - macOS: Prepend clawproxy-run to ProgramArguments array
    - Linux: Prepend clawproxy-run to ExecStart line
12. Add service dependency:
    - macOS: (launchd doesn't support dependencies well, rely on KeepAlive retry)
    - Linux: Add After=clawproxy.service and Requires=clawproxy.service to [Unit]
13. Write modified file
14. Reload the service:
    - macOS: launchctl unload && launchctl load
    - Linux: systemctl --user daemon-reload && systemctl --user restart openclaw-gateway

Part C: Verification
15. Verify ClawProxy is running (warn if not)
16. Verify sandbox is active (test that secrets dir is blocked)
17. Print summary:
    - Files modified (with backup locations)
    - Secrets migrated
    - Next steps / verification commands

Flags:
- --dry-run: Show what would be changed without modifying files
- --revert: Restore all files from .pre-clawproxy backups
- --skip-config: Only modify daemon, don't touch openclaw.json
- --skip-daemon: Only modify config, don't touch service file
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



---

## ClawProxy + OpenClaw Integration

### Overview

This section describes how to integrate ClawProxy with OpenClaw to protect API credentials from prompt injection attacks while maintaining OpenClaw's existing functionality.

---

### Quick Start

**Prerequisites:** OpenClaw must already be installed and configured (`openclaw onboard` completed).

```bash
# 1. Initialize ClawProxy
clawproxy init

# 2. Integrate with OpenClaw (migrates keys, updates configs, modifies daemon)
clawproxy configure-openclaw

# 3. Start ClawProxy
clawproxy start
# Or: launchctl load ~/Library/LaunchAgents/ai.clawproxy.plist (macOS)
# Or: systemctl --user start clawproxy (Linux)

# 4. Verify
clawproxy configure-openclaw --dry-run  # Should show "already configured"
openclaw health                          # Should work normally
```

The `configure-openclaw` command handles everything:
- Migrates existing API keys from OpenClaw config to ClawProxy secrets (with prompts)
- Updates `~/.openclaw/openclaw.json` to route through the proxy
- Modifies the OpenClaw daemon to run under `clawproxy-run` sandbox
- Reloads the daemon service

---

### 1. What the Integration Changes (Reference)

#### Provider Configuration

The `configure-openclaw` command modifies `~/.openclaw/openclaw.json`:

```json
{
  "models": {
    "providers": {
      "anthropic": {
        "baseUrl": "http://127.0.0.1:8080/anthropic",
        "apiKey": "PROXY:anthropic"
      },
      "openai": {
        "baseUrl": "http://127.0.0.1:8080/openai",
        "apiKey": "PROXY:openai"
      }
    }
  },
  "agent": {
    "model": "anthropic/claude-sonnet-4-5"
  }
}
```

**Key points:**
- `baseUrl` points to ClawProxy instead of the provider's API
- `apiKey` uses placeholder tokens (`PROXY:xxx`) - the real keys are in ClawProxy's secrets
- Original config backed up to `~/.openclaw/openclaw.json.pre-clawproxy`

**How the auth header works:**

OpenClaw's provider SDKs automatically add auth headers using the configured `apiKey` value. For example, with `apiKey: "PROXY:openai"`:

1. OpenClaw's OpenAI SDK sends: `Authorization: Bearer PROXY:openai`
2. ClawProxy intercepts this request (via the modified `baseUrl`)
3. ClawProxy substitutes the placeholder: `PROXY:openai` → `sk-xxxxxxxx`
4. ClawProxy forwards to `https://api.openai.com` with the real key

No additional configuration is needed - the placeholder token flows through the existing auth mechanism.

#### Credential Migration

The configure-openclaw command will prompt to migrate any existing keys it finds:
```
Found anthropic API key in openclaw.json. Migrate to ClawProxy? [Y/n]
```

If you need to add keys manually later:
```bash
clawproxy secret set anthropic   # Enter your Anthropic key
clawproxy secret set openai      # Enter your OpenAI key
```

---

### 2. Daemon Service Modification (Reference)

The `configure-openclaw` command modifies the daemon configuration so the sandbox persists across restarts. This is necessary because the daemon restarts the gateway process on crashes and reboots.

**Why this matters:** If you only run `clawproxy-run` manually, the daemon will restart the process *without* the sandbox.

**Reverting:** Use `clawproxy configure-openclaw --revert` to restore original files from `.pre-clawproxy` backups.

**macOS (launchd):**

The OpenClaw gateway plist (`~/Library/LaunchAgents/ai.openclaw.gateway.plist`) is modified:

```xml
<!-- Before -->
<key>ProgramArguments</key>
<array>
    <string>/usr/local/bin/node</string>
    <string>/path/to/openclaw/dist/index.js</string>
    <string>gateway</string>
    <string>--port</string>
    <string>18789</string>
</array>

<!-- After -->
<key>ProgramArguments</key>
<array>
    <string>/usr/local/bin/clawproxy-run</string>
    <string>/usr/local/bin/node</string>
    <string>/path/to/openclaw/dist/index.js</string>
    <string>gateway</string>
    <string>--port</string>
    <string>18789</string>
</array>
```

**Linux (systemd):**

The OpenClaw service (`~/.config/systemd/user/openclaw-gateway.service`) is modified:

```ini
# Before
[Service]
ExecStart=/usr/local/bin/node /path/to/openclaw/dist/index.js gateway --port 18789

# After
[Unit]
After=clawproxy.service
Requires=clawproxy.service

[Service]
ExecStart=/usr/local/bin/clawproxy-run /usr/local/bin/node /path/to/openclaw/dist/index.js gateway --port 18789
```

---

### 3. Docker Sandbox Considerations

When OpenClaw uses Docker for tool sandboxing, the containers need to reach ClawProxy on the host.

#### Network Access

Docker containers can reach host services via `host.docker.internal` (macOS/Windows) or the host's IP on the `docker0` bridge (Linux).

**Option A: Configure OpenClaw to use host.docker.internal**

If OpenClaw's Docker sandbox inherits the proxy environment or makes its own API calls:

```json
{
  "models": {
    "providers": {
      "anthropic": {
        "baseUrl": "http://host.docker.internal:8080/anthropic",
        "apiKey": "PROXY:anthropic"
      }
    }
  }
}
```

**Option B: ClawProxy binds to all interfaces**

If containers can't resolve `host.docker.internal`, configure ClawProxy to listen on `0.0.0.0`:

```yaml
# ~/.config/clawproxy/config.yaml
listen:
  host: "0.0.0.0"  # Instead of 127.0.0.1
  port: 8080
```

**Security note:** Only do this on trusted networks. Consider firewall rules to restrict access.

#### Sandbox Isolation

The ClawProxy secrets directory (`~/.config/clawproxy/secrets/`) should NOT be mounted into Docker containers. Verify OpenClaw's sandbox config doesn't expose this path.

---

### 4. Startup Order and Dependencies

ClawProxy must be running before OpenClaw starts, otherwise API requests will fail.

#### Recommended Service Dependencies

The `clawproxy init` command creates the necessary service files automatically:

**macOS (launchd):**

ClawProxy service file (`~/Library/LaunchAgents/ai.clawproxy.plist`) is created by `clawproxy init`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>ai.clawproxy</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/clawproxy</string>
        <string>start</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/usr/local/var/log/clawproxy.log</string>
    <key>StandardErrorPath</key>
    <string>/usr/local/var/log/clawproxy.log</string>
</dict>
</plist>
```

Add dependency to OpenClaw's plist so it waits for ClawProxy:
```xml
<key>RunAtLoad</key>
<true/>
<key>KeepAlive</key>
<dict>
    <key>NetworkState</key>
    <true/>
</dict>
```

**Linux (systemd):**

ClawProxy service file (`~/.config/systemd/user/clawproxy.service`) is created by `clawproxy init`:
```ini
[Unit]
Description=ClawProxy - Credential injection proxy for AI agents

[Service]
ExecStart=/usr/local/bin/clawproxy start
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

Add dependency to OpenClaw's service file:
```ini
# openclaw-gateway.service
[Unit]
After=clawproxy.service
Requires=clawproxy.service

[Service]
ExecStart=/usr/local/bin/clawproxy-run /usr/local/bin/node /path/to/openclaw/dist/index.js gateway --port 18789
```

**Starting the services:**
```bash
# macOS
launchctl load ~/Library/LaunchAgents/ai.clawproxy.plist

# Linux
systemctl --user start clawproxy
```

#### Manual Startup

```bash
# Terminal 1 (or run as service)
clawproxy start

# Terminal 2: Run gateway manually with sandbox
clawproxy-run openclaw gateway --port 18789

# Or restart the daemon service (if using launchd/systemd)
# macOS: via OpenClaw app or launchctl
# Linux: systemctl --user restart openclaw-gateway
```

---

### 5. Verification

#### Test ClawProxy is Running

```bash
curl http://127.0.0.1:8080/health
# Should return OK or similar
```

#### Test Sandbox is Applied

```bash
# This should fail with "Operation not permitted"
clawproxy-run cat ~/.config/clawproxy/secrets/anthropic
```

#### Test OpenClaw Integration

```bash
# Check gateway health
openclaw health

# Or send a test message via the agent command
openclaw agent --message "Say hello"
# Should work normally - API calls are proxied through ClawProxy
```

#### Verify No Real Keys in OpenClaw

```bash
# Check for any leaked keys
grep -r "sk-" ~/.openclaw/
# Should only find PROXY:xxx placeholders, not real keys
```

---

### 6. Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "Connection refused" errors | ClawProxy not running | Start `clawproxy start` first |
| "PROXY:xxx is not a valid API key" | Upstream receiving placeholder | Check baseUrl points to ClawProxy, not provider |
| OpenClaw can read secrets | Sandbox not applied | Verify daemon runs via `clawproxy-run` |
| Docker tools fail to call APIs | Can't reach host proxy | Use `host.docker.internal` or bind to 0.0.0.0 |

---

### Open Questions for Integration

1. **OpenClaw's exact credential paths**: Need to verify all locations where OpenClaw might store/read API keys to ensure they're migrated to ClawProxy.

2. **OAuth flow**: OpenClaw supports OAuth for Anthropic/OpenAI subscriptions. Does this flow work through a proxy, or does it need special handling?

3. **`openclaw onboard` modifications**: Should ClawProxy integration be part of onboarding, or a separate setup step?

4. **Graceful degradation**: If ClawProxy isn't running, should OpenClaw fail fast or provide a helpful error message?

---

## Open Questions

1. **Daemon mode**: Should `clawproxy start` support `-d` for daemon mode, or rely on systemd/launchd?

2. **Multiple configs**: Support `--profile` flag for different configs (work vs personal)?

3. **Proxy authentication**: Should the proxy itself require auth from the agent (defense in depth)?

4. **Audit logging**: Log all requests to a file for security review?

5. **Graceful sandbox fallback**: On older Linux, should we refuse to run (secure) or warn and continue (usable)?
